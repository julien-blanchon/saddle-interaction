//! # FPS Interactive — Cross-Crate Example
//!
//! Combines **saddle-character-controller** (physics-based FPS movement via
//! Avian3D) with **saddle-interaction** (world-interaction prompts). Walk
//! around a room with a door, a switch, and a console using real physics.
//!
//! Press **F** to interact (E is used by the character controller for traverse).
//!
//! **Cross-crate integration points**:
//! - `CharacterController` + `Interactor` on the same player entity
//! - `InteractorAim` synced from camera forward each frame
//! - Interaction prompt floats above focused targets

use bevy::prelude::*;
use bevy_enhanced_input::prelude::{Cancel as InputCancel, *};
use avian3d::prelude::*;
use saddle_character_controller::{
    CharacterController, CharacterControllerPlugin, CharacterFlying, CharacterPush,
};
use saddle_interaction::{
    ActiveInteraction, FocusedInteraction, Interactable, InteractionBehavior,
    InteractionExecution, InteractionIntent, InteractionIntentKind,
    InteractionPlugin, InteractionPromptState, InteractionSlot, InteractionTarget,
    InteractionTags, Interactor, InteractorAim,
};

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

#[derive(Component)]
struct FpsPlayer;

#[derive(Component)]
struct FpsInputContext;

#[derive(Component)]
struct PromptLabel;

#[derive(Component)]
struct TargetHighlight {
    base_color: Color,
    handle: Handle<StandardMaterial>,
}

// ---------------------------------------------------------------------------
// Input actions
// ---------------------------------------------------------------------------

#[derive(InputAction)]
#[action_output(bool)]
struct InteractAction;

#[derive(InputAction)]
#[action_output(bool)]
struct CancelAction;

// Character controller actions (from its enhanced-input adapter)
use saddle_character_controller::adapters::enhanced_input::{
    CharacterControllerEnhancedInputPlugin, MoveAction, JumpAction, SprintAction, CrouchAction,
};

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> AppExit {
    let mut app = App::new();

    app.insert_resource(ClearColor(Color::srgb(0.12, 0.12, 0.14)));
    app.insert_resource(Time::<Fixed>::from_hz(60.0));

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "interaction / fps_interactive".into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }));

    // Physics
    app.add_plugins(PhysicsPlugins::default());

    // Enhanced input
    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<FpsInputContext>();

    // Character controller (physics-based FPS movement)
    app.add_plugins((
        CharacterControllerPlugin::always_on(FixedUpdate),
        CharacterControllerEnhancedInputPlugin,
    ));

    // Interaction plugin
    app.add_plugins(InteractionPlugin::default());

    // Input → interaction intent observers
    app.add_observer(on_interact_start);
    app.add_observer(on_interact_release);
    app.add_observer(on_interact_cancel);

    app.add_systems(Startup, setup);
    app.add_systems(Update, (
        sync_interactor_aim,
        tint_targets,
        update_prompt,
    ));

    app.run()
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Lighting
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.9, 0.4, 0.0)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.7, 0.75, 0.85),
        brightness: 250.0,
        affects_lightmapped_meshes: false,
    });

    // Ground (physics collider)
    commands.spawn((
        Name::new("Ground"),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(50.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.26, 0.28),
            perceptual_roughness: 0.9,
            ..default()
        })),
    ));

    // --- Player (character controller + interactor) ---
    let player = commands.spawn((
        Name::new("Player"),
        FpsPlayer,
        FpsInputContext,
        // Character controller
        CharacterController {
            speed: 8.0,
            ..default()
        },
        CharacterFlying::default(),
        CharacterPush::default(),
        // Interaction
        Interactor {
            max_distance: Some(5.0),
            proximity_radius: Some(5.0),
            alignment_weight: 0.8,
            distance_weight: 1.0,
            require_line_of_sight: false,
            ..default()
        },
        InteractionTags::default(),
        InteractorAim { direction: Vec3::NEG_Z },
        // Transform
        Transform::from_xyz(0.0, 2.0, 8.0),
        Visibility::Inherited,
        // Input bindings (F for interact, Escape for cancel + standard movement)
        actions!(FpsInputContext[
            (Action::<InteractAction>::new(), bindings![KeyCode::KeyF, GamepadButton::West]),
            (Action::<CancelAction>::new(), bindings![KeyCode::Escape, GamepadButton::East]),
            (
                Action::<MoveAction>::new(),
                DeadZone::default(),
                Bindings::spawn((Cardinal::wasd_keys(), Axial::left_stick())),
            ),
            (Action::<JumpAction>::new(), bindings![KeyCode::Space, GamepadButton::South]),
            (Action::<SprintAction>::new(), bindings![KeyCode::ShiftLeft, GamepadButton::LeftTrigger2]),
            (Action::<CrouchAction>::new(), bindings![KeyCode::ControlLeft, GamepadButton::RightTrigger]),
        ]),
    )).id();

    // Camera as child of player
    let camera = commands.spawn((
        Name::new("FPS Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.6, 0.0),
    )).id();
    commands.entity(player).add_child(camera);

    // --- Interactable objects ---

    // Door (instant)
    let door_color = Color::srgb(0.55, 0.35, 0.15);
    let door_handle = materials.add(StandardMaterial {
        base_color: door_color,
        perceptual_roughness: 0.6,
        ..default()
    });
    let slots = vec![InteractionSlot::instant("open", "Open Door")];
    commands.spawn((
        Name::new("Door"),
        Mesh3d(meshes.add(Cuboid::new(2.0, 3.0, 0.3))),
        MeshMaterial3d(door_handle.clone()),
        Transform::from_xyz(-4.0, 1.5, 0.0),
        TargetHighlight { base_color: door_color, handle: door_handle },
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // Switch (toggle)
    let switch_color = Color::srgb(0.2, 0.5, 0.3);
    let switch_handle = materials.add(StandardMaterial {
        base_color: switch_color,
        perceptual_roughness: 0.5,
        ..default()
    });
    let slots = vec![InteractionSlot {
        behavior: InteractionBehavior::Single(InteractionExecution::Toggle),
        ..InteractionSlot::instant("toggle", "Toggle Switch")
    }];
    commands.spawn((
        Name::new("Switch"),
        Mesh3d(meshes.add(Cuboid::new(0.3, 0.4, 0.1))),
        MeshMaterial3d(switch_handle.clone()),
        Transform::from_xyz(0.0, 1.2, -2.0),
        TargetHighlight { base_color: switch_color, handle: switch_handle },
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // Console (hold)
    let console_color = Color::srgb(0.15, 0.35, 0.65);
    let console_handle = materials.add(StandardMaterial {
        base_color: console_color,
        perceptual_roughness: 0.5,
        ..default()
    });
    let slots = vec![InteractionSlot {
        behavior: InteractionBehavior::Single(InteractionExecution::Hold {
            duration_seconds: 1.5,
        }),
        ..InteractionSlot::instant("hack", "Hack Console")
    }];
    commands.spawn((
        Name::new("Console"),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.2, 0.5))),
        MeshMaterial3d(console_handle.clone()),
        Transform::from_xyz(4.0, 0.6, 0.0),
        TargetHighlight { base_color: console_color, handle: console_handle },
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // Prompt UI
    commands.spawn((
        Name::new("Prompt"),
        PromptLabel,
        Text::new(""),
        TextFont { font_size: 20.0, ..default() },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(40.0),
            left: Val::Percent(50.0),
            ..default()
        },
    ));
}

// ---------------------------------------------------------------------------
// Input → intent observers
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

// ---------------------------------------------------------------------------
// Sync interactor aim from camera forward
// ---------------------------------------------------------------------------

fn sync_interactor_aim(
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut players: Query<&mut InteractorAim, With<FpsPlayer>>,
) {
    let Ok(cam_gt) = cameras.single() else { return };
    for mut aim in &mut players {
        aim.direction = cam_gt.forward().into();
    }
}

// ---------------------------------------------------------------------------
// Visual feedback
// ---------------------------------------------------------------------------

fn tint_targets(
    interactors: Query<(&FocusedInteraction, Option<&ActiveInteraction>), With<FpsPlayer>>,
    targets: Query<(Entity, &TargetHighlight)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let focused = interactors.iter().next().and_then(|(f, _)| f.target);
    let active = interactors.iter().next().and_then(|(_, a)| a).is_some();

    for (entity, highlight) in &targets {
        let Some(mat) = materials.get_mut(&highlight.handle) else { continue; };
        if Some(entity) == focused {
            mat.base_color = if active {
                Color::srgb(0.3, 0.85, 0.4)
            } else {
                Color::srgb(0.9, 0.7, 0.2)
            };
        } else {
            mat.base_color = highlight.base_color;
        }
    }
}

fn update_prompt(
    interactors: Query<&InteractionPromptState, With<FpsPlayer>>,
    mut label: Query<&mut Text, With<PromptLabel>>,
) {
    let Ok(mut text) = label.single_mut() else { return; };
    let prompt = interactors
        .iter()
        .next()
        .and_then(|s| s.offer.as_ref())
        .map(|o| format!("[F] {}", o.prompt.action_label_key));

    **text = prompt.unwrap_or_default();
}
