//! Shared utilities for saddle-interaction examples.
//!
//! Provides a pre-configured [`base_app`], FPS-style player controller, scene
//! helpers, world-space interaction prompts, visual feedback, and a diagnostics
//! overlay so that individual examples only need to set up their scene-specific
//! interactable entities.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::{Cancel as InputCancel, *};
use saddle_interaction::{
    ActiveInteraction, FocusedInteraction, Interactable, InteractionBehavior,
    InteractionCanceled, InteractionCompleted, InteractionConfig, InteractionExecution,
    InteractionIntent, InteractionIntentKind, InteractionOffered,
    InteractionPlugin, InteractionPromptState, InteractionSlot, InteractionTarget, Interactor,
    InteractionTags,
};
use saddle_pane::prelude::*;

// ---------------------------------------------------------------------------
// Re-exports used by examples
// ---------------------------------------------------------------------------

pub use saddle_interaction;

// ---------------------------------------------------------------------------
// Input actions (shared across all examples — eliminates per-example duplication)
// ---------------------------------------------------------------------------

#[derive(InputAction)]
#[action_output(bool)]
pub struct InteractAction;

#[derive(InputAction)]
#[action_output(bool)]
pub struct CancelAction;

#[derive(InputAction)]
#[action_output(bool)]
pub struct NextSlotAction;

#[derive(InputAction)]
#[action_output(bool)]
pub struct PrevSlotAction;

/// Input context marker for the demo player.
#[derive(Component)]
pub struct DemoInputContext;

// ---------------------------------------------------------------------------
// Player components
// ---------------------------------------------------------------------------

/// Marker for the demo player entity.
#[derive(Component)]
pub struct DemoPlayer;

/// Simple FPS-style controller state (transform-based, no physics).
#[derive(Component)]
pub struct DemoPlayerController {
    pub speed: f32,
    pub sensitivity: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for DemoPlayerController {
    fn default() -> Self {
        Self {
            speed: 5.0,
            sensitivity: 0.003,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Target components
// ---------------------------------------------------------------------------

/// Stores the original slot configuration so the pane can reset tuned values.
#[derive(Component, Clone)]
pub struct DemoBaseTargetSlots(pub Vec<InteractionSlot>);

/// Stores material colors for visual feedback on targets.
#[derive(Component)]
pub struct DemoTargetColors {
    pub base: Color,
    pub focused: Color,
    pub active: Color,
    pub handle: Handle<StandardMaterial>,
}

// ---------------------------------------------------------------------------
// Prompt UI
// ---------------------------------------------------------------------------

#[derive(Component)]
struct PromptUiRoot;

#[derive(Component)]
struct PromptUiText;

// ---------------------------------------------------------------------------
// Diagnostics overlay
// ---------------------------------------------------------------------------

#[derive(Component)]
struct OverlayRoot;

/// Diagnostics state tracked for the overlay (and available to examples).
#[derive(Resource, Default, Debug, Clone)]
pub struct DemoDiagnostics {
    pub focused_target: Option<String>,
    pub prompt_label: Option<String>,
    pub active_slot: Option<String>,
    pub active_progress: f32,
    pub completed_count: usize,
    pub canceled_count: usize,
    pub last_result: String,
}

// ---------------------------------------------------------------------------
// Prop shapes
// ---------------------------------------------------------------------------

/// Shape primitives for [`spawn_prop`].
pub enum PropShape {
    Cube(Vec3),
    Cylinder { radius: f32, height: f32 },
    Sphere(f32),
}

// ---------------------------------------------------------------------------
// Pane (live tuning)
// ---------------------------------------------------------------------------

#[derive(Resource, Clone, Default, Pane)]
#[pane(title = "Interaction Tuning")]
pub struct InteractionDemoPane {
    #[pane(slider, min = 1.0, max = 20.0, step = 0.5)]
    pub actor_range: f32,
    #[pane(slider, min = 0.5, max = 2.5, step = 0.05)]
    pub detection_radius_scale: f32,
    #[pane(slider, min = 0.25, max = 2.5, step = 0.05)]
    pub hold_time_scale: f32,
    pub hold_to_toggle: bool,
    pub auto_interact_on_focus: bool,
}

impl InteractionDemoPane {
    pub fn new() -> Self {
        Self {
            actor_range: 6.0,
            detection_radius_scale: 1.0,
            hold_time_scale: 1.0,
            hold_to_toggle: false,
            auto_interact_on_focus: false,
        }
    }
}

// ===================================================================
// PUBLIC API
// ===================================================================

/// Create a pre-configured [`App`] with everything wired up.
///
/// The returned app has: DefaultPlugins, enhanced input, interaction plugin,
/// FPS player controller systems, world-space prompts, visual feedback,
/// diagnostics overlay, and the interaction tuning pane.
///
/// Each example only needs to add `Startup` systems to spawn interactable
/// entities and `Update` systems for game-specific responses (e.g.,
/// reacting to `InteractionCompleted`).
pub fn base_app(title: &str) -> App {
    let mut app = App::new();

    app.insert_resource(ClearColor(Color::srgb(0.12, 0.12, 0.14)));
    app.init_resource::<DemoDiagnostics>();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: title.into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }));

    // Pane plugins
    app.add_plugins((
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    ));
    app.insert_resource(InteractionDemoPane::new());
    app.register_pane::<InteractionDemoPane>();

    // Enhanced input
    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<DemoInputContext>();

    // Interaction plugin
    app.add_plugins(InteractionPlugin::default());

    // Input → intent observers
    app.add_observer(on_interact_start);
    app.add_observer(on_interact_release);
    app.add_observer(on_interact_cancel);
    app.add_observer(on_explicit_cancel);
    app.add_observer(on_next_slot);
    app.add_observer(on_prev_slot);

    // Systems
    app.add_systems(Startup, spawn_prompt_ui);
    app.add_systems(Startup, spawn_diagnostics_overlay);
    app.add_systems(
        Update,
        (
            move_player,
            update_interactor_aim,
            tint_focused_targets,
            update_prompt_ui,
            record_interaction_events,
            update_diagnostics_overlay,
            sync_pane,
        ),
    );

    app
}

/// Spawn a ground plane, directional light, and ambient light.
pub fn spawn_environment(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Ground plane
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(50.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.26, 0.28),
            perceptual_roughness: 0.9,
            ..default()
        })),
    ));

    // Directional light (sun)
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.4, 0.0)),
    ));

    // Global ambient light
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.7, 0.75, 0.85),
        brightness: 250.0,
        affects_lightmapped_meshes: false,
    });
}

/// Spawn a 3D prop entity with a mesh and material. Returns the entity so
/// the caller can insert interaction components.
pub fn spawn_prop(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    name: &str,
    shape: PropShape,
    position: Vec3,
    color: Color,
) -> Entity {
    let mesh = match shape {
        PropShape::Cube(size) => meshes.add(Cuboid::new(size.x, size.y, size.z)),
        PropShape::Cylinder { radius, height } => {
            meshes.add(Cylinder::new(radius, height))
        }
        PropShape::Sphere(radius) => meshes.add(Sphere::new(radius)),
    };

    let focused = adjust_brightness(color, 1.6);
    let active = Color::srgb(0.3, 0.85, 0.4);
    let handle = materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.6,
        ..default()
    });

    commands
        .spawn((
            Name::new(name.to_owned()),
            Mesh3d(mesh),
            MeshMaterial3d(handle.clone()),
            Transform::from_translation(position),
            DemoTargetColors {
                base: color,
                focused,
                active,
                handle,
            },
        ))
        .id()
}

/// Spawn the FPS-style demo player at the given position.
///
/// The player entity has an [`Interactor`], [`InteractionTags`], and a child
/// [`Camera3d`]. It uses WASD for movement and mouse for look.
pub fn spawn_player(commands: &mut Commands, position: Vec3) -> Entity {
    let player = commands
        .spawn((
            Name::new("Player"),
            DemoPlayer,
            DemoInputContext,
            DemoPlayerController::default(),
            Interactor {
                max_distance: Some(6.0),
                proximity_radius: Some(6.0),
                alignment_weight: 0.6,
                distance_weight: 1.0,
                target_priority_weight: 0.8,
                require_line_of_sight: false,
                ..default()
            },
            InteractionTags::default(),
            Transform::from_translation(position),
            Visibility::default(),
            // Input bindings
            actions!(DemoInputContext[
                (Action::<InteractAction>::new(), bindings![KeyCode::KeyE, GamepadButton::South]),
                (Action::<CancelAction>::new(), bindings![KeyCode::Escape, GamepadButton::East]),
                (Action::<NextSlotAction>::new(), bindings![KeyCode::Tab]),
                (Action::<PrevSlotAction>::new(), bindings![KeyCode::KeyQ]),
            ]),
        ))
        .id();

    // Camera as child
    let camera = commands
        .spawn((
            Name::new("Player Camera"),
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    commands.entity(player).add_child(camera);
    player
}

// ===================================================================
// INTERNAL SYSTEMS
// ===================================================================

// ---------------------------------------------------------------------------
// Input → InteractionIntent observers
// ---------------------------------------------------------------------------

fn on_interact_start(
    trigger: On<Start<InteractAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::Press,
    });
}

fn on_interact_release(
    trigger: On<Complete<InteractAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::Release,
    });
}

fn on_interact_cancel(
    trigger: On<InputCancel<InteractAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::Cancel,
    });
}

fn on_explicit_cancel(
    trigger: On<Start<CancelAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::Cancel,
    });
}

fn on_next_slot(
    trigger: On<Start<NextSlotAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::CycleNext,
    });
}

fn on_prev_slot(
    trigger: On<Start<PrevSlotAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::CyclePrevious,
    });
}

// ---------------------------------------------------------------------------
// FPS player movement (transform-based, no physics)
// ---------------------------------------------------------------------------

fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut DemoPlayerController), With<DemoPlayer>>,
    time: Res<Time>,
    accumulated: Res<AccumulatedMouseMotion>,
) {
    let Ok((mut transform, mut ctrl)) = query.single_mut() else {
        return;
    };

    // Mouse look
    let delta = accumulated.delta;
    if delta != Vec2::ZERO {
        ctrl.yaw -= delta.x * ctrl.sensitivity;
        ctrl.pitch = (ctrl.pitch - delta.y * ctrl.sensitivity).clamp(-1.4, 1.4);
    }

    // Movement direction relative to yaw
    let mut dir = Vec3::ZERO;
    let forward = Vec3::new(ctrl.yaw.sin(), 0.0, ctrl.yaw.cos());
    let right = Vec3::new(forward.z, 0.0, -forward.x);

    if keys.pressed(KeyCode::KeyW) {
        dir -= forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir += forward;
    }
    if keys.pressed(KeyCode::KeyA) {
        dir -= right;
    }
    if keys.pressed(KeyCode::KeyD) {
        dir += right;
    }

    if dir.length_squared() > 0.0 {
        dir = dir.normalize();
    }

    transform.translation += dir * ctrl.speed * time.delta_secs();
    // Keep player at eye height
    transform.translation.y = transform.translation.y.max(1.6);

    // Apply rotation
    transform.rotation = Quat::from_euler(EulerRot::YXZ, ctrl.yaw, ctrl.pitch, 0.0);
}

fn update_interactor_aim(
    mut query: Query<(&Transform, &mut saddle_interaction::InteractorAim), With<DemoPlayer>>,
) {
    for (transform, mut aim) in &mut query {
        aim.direction = transform.forward().into();
    }
}

// ---------------------------------------------------------------------------
// Visual feedback: tint targets when focused/active
// ---------------------------------------------------------------------------

fn tint_focused_targets(
    interactors: Query<(&FocusedInteraction, Option<&ActiveInteraction>), With<DemoPlayer>>,
    targets: Query<(Entity, &DemoTargetColors)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let focused_target = interactors
        .iter()
        .next()
        .and_then(|(f, _)| f.target);
    let has_active = interactors
        .iter()
        .next()
        .and_then(|(_, a)| a)
        .is_some();

    for (entity, colors) in &targets {
        let Some(mat) = materials.get_mut(&colors.handle) else {
            continue;
        };
        if Some(entity) == focused_target {
            mat.base_color = if has_active { colors.active } else { colors.focused };
        } else {
            mat.base_color = colors.base;
        }
    }
}

// ---------------------------------------------------------------------------
// World-space prompt UI
// ---------------------------------------------------------------------------

fn spawn_prompt_ui(mut commands: Commands) {
    commands.spawn((
        Name::new("Prompt UI"),
        PromptUiRoot,
        PromptUiText,
        Text::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(16.0), px(8.0)),
            ..default()
        },
        Visibility::Hidden,
        GlobalZIndex(10),
    ));
}

fn update_prompt_ui(
    interactors: Query<(&InteractionPromptState, &FocusedInteraction), With<DemoPlayer>>,
    targets: Query<&GlobalTransform>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut prompt: Query<(&mut Node, &mut Visibility, &mut Text), With<PromptUiRoot>>,
) {
    let Ok((mut node, mut vis, mut text)) = prompt.single_mut() else {
        return;
    };

    // Get current prompt info
    let prompt_info = interactors.iter().next().and_then(|(state, focus)| {
        let offer = state.offer.as_ref()?;
        let target = focus.target?;
        Some((offer.prompt.action_label_key.clone(), target))
    });

    let Some((label, target_entity)) = prompt_info else {
        *vis = Visibility::Hidden;
        return;
    };

    // Get target world position
    let Ok(target_transform) = targets.get(target_entity) else {
        *vis = Visibility::Hidden;
        return;
    };

    // Project to screen
    let Ok((camera, camera_transform)) = cameras.single() else {
        *vis = Visibility::Hidden;
        return;
    };

    let target_pos = target_transform.translation() + Vec3::Y * 1.5;
    let Ok(viewport_pos) = camera.world_to_viewport(camera_transform, target_pos) else {
        *vis = Visibility::Hidden;
        return;
    };

    *vis = Visibility::Inherited;
    **text = format!("Press E — {label}");
    node.left = px(viewport_pos.x - 80.0);
    node.top = px(viewport_pos.y - 20.0);
}

// ---------------------------------------------------------------------------
// Diagnostics overlay
// ---------------------------------------------------------------------------

fn spawn_diagnostics_overlay(mut commands: Commands) {
    commands.spawn((
        Name::new("Diagnostics Overlay"),
        OverlayRoot,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            bottom: px(16.0),
            max_width: px(380.0),
            ..default()
        },
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
    ));
}

fn record_interaction_events(
    mut diag: ResMut<DemoDiagnostics>,
    mut completed: MessageReader<InteractionCompleted>,
    mut canceled: MessageReader<InteractionCanceled>,
    mut offered: MessageReader<InteractionOffered>,
    interactors: Query<
        (&FocusedInteraction, &InteractionPromptState, Option<&ActiveInteraction>),
        With<DemoPlayer>,
    >,
    names: Query<&Name>,
) {
    // Update focus/prompt state from components
    if let Some((focus, prompt_state, active)) = interactors.iter().next() {
        diag.focused_target = focus
            .target
            .and_then(|e| names.get(e).ok())
            .map(|n| n.to_string());
        diag.prompt_label = prompt_state
            .offer
            .as_ref()
            .map(|o| o.prompt.action_label_key.clone());
        if let Some(a) = active {
            diag.active_slot = Some(a.slot_id.0.clone());
            diag.active_progress = a.progress;
        } else {
            diag.active_slot = None;
            diag.active_progress = 0.0;
        }
    }

    for event in completed.read() {
        diag.completed_count += 1;
        diag.last_result = format!("completed: {}", event.slot_id.0);
    }

    for event in canceled.read() {
        diag.canceled_count += 1;
        diag.last_result = format!("canceled: {:?}", event.reason);
    }

    // Drain offered events
    for _ in offered.read() {}
}

fn update_diagnostics_overlay(
    diag: Res<DemoDiagnostics>,
    mut overlay: Query<&mut Text, With<OverlayRoot>>,
) {
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };

    let focus = diag
        .focused_target
        .as_deref()
        .unwrap_or("none");
    let prompt = diag
        .prompt_label
        .as_deref()
        .unwrap_or("none");
    let active = diag
        .active_slot
        .as_deref()
        .unwrap_or("none");

    **text = format!(
        " focus: {focus}  prompt: {prompt}\n \
         active: {active}  progress: {:.0}%\n \
         completed: {}  canceled: {}\n \
         {}",
        diag.active_progress * 100.0,
        diag.completed_count,
        diag.canceled_count,
        if diag.last_result.is_empty() {
            String::new()
        } else {
            format!(" last: {}", diag.last_result)
        },
    );
}

// ---------------------------------------------------------------------------
// Pane sync
// ---------------------------------------------------------------------------

fn sync_pane(
    pane: Res<InteractionDemoPane>,
    mut config: ResMut<InteractionConfig>,
    mut interactors: Query<&mut Interactor, With<DemoPlayer>>,
    mut targets: Query<(
        &mut Interactable,
        &mut InteractionTarget,
        &DemoBaseTargetSlots,
    )>,
) {
    if !pane.is_changed() {
        return;
    }

    config.hold_to_toggle = pane.hold_to_toggle;
    config.auto_interact_on_focus = pane.auto_interact_on_focus;
    config.detection_radius_scale = pane.detection_radius_scale;
    config.hold_time_scale = pane.hold_time_scale;

    for mut interactor in &mut interactors {
        interactor.max_distance = Some(pane.actor_range);
        interactor.proximity_radius = Some(pane.actor_range);
    }

    for (mut interactable, mut target, base_slots) in &mut targets {
        interactable.focus_radius = Some(pane.actor_range);
        target.slots = base_slots.0.clone();
        for slot in &mut target.slots {
            slot.auto_trigger_on_focus = pane.auto_interact_on_focus;
            scale_hold_durations(&mut slot.behavior, pane.hold_time_scale);
        }
    }
}

fn scale_hold_durations(behavior: &mut InteractionBehavior, scale: f32) {
    match behavior {
        InteractionBehavior::Single(InteractionExecution::Hold { duration_seconds })
        | InteractionBehavior::Single(InteractionExecution::Passive { duration_seconds }) => {
            *duration_seconds *= scale;
        }
        InteractionBehavior::Single(_) => {}
        InteractionBehavior::Sequence { stages, .. } => {
            for stage in stages {
                match &mut stage.execution {
                    InteractionExecution::Hold { duration_seconds }
                    | InteractionExecution::Passive { duration_seconds } => {
                        *duration_seconds *= scale;
                    }
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn adjust_brightness(color: Color, factor: f32) -> Color {
    let linear = color.to_linear();
    Color::LinearRgba(LinearRgba::new(
        (linear.red * factor).min(1.0),
        (linear.green * factor).min(1.0),
        (linear.blue * factor).min(1.0),
        linear.alpha,
    ))
}

fn px(val: f32) -> Val {
    Val::Px(val)
}
