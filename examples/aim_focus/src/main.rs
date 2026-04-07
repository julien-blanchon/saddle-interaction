//! # Aim-Focus and Dynamic Control Example
//!
//! Demonstrates two common patterns:
//!
//! ## 1. Crosshair-style aim-to-focus
//!
//! Only the object at the **center of the camera view** gets focused. This is
//! achieved by cranking `alignment_weight` high and reducing `distance_weight`,
//! so objects you're not looking at score too low to win focus — even if closer.
//!
//! A center-screen crosshair shows what you're aiming at. Walk around the room
//! with four objects and notice the prompt only appears when you look directly
//! at one.
//!
//! ## 2. Dynamic enable/disable
//!
//! Press **1** to toggle the red pillar's `Interactable.enabled` flag. When
//! disabled, the object is skipped entirely by the detection pipeline — no
//! focus, no prompt, no interaction. Press **1** again to re-enable it.
//!
//! Press **2** to toggle the green sphere between instant and hold behaviors,
//! showing runtime slot reconfiguration.
//!
//! **Concepts**: `alignment_weight`, `InteractorAim`, `Interactable.enabled`,
//! runtime slot mutation.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use saddle_interaction::{
    Interactable, InteractionBehavior, InteractionCompleted, InteractionExecution,
    InteractionSlot, InteractionTarget,
};
use saddle_interaction_example_common as common;
use common::{DemoBaseTargetSlots, DemoTargetColors};

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marks the red pillar that can be toggled on/off.
#[derive(Component)]
struct ToggleTarget;

/// Marks the green sphere whose slot behavior can be swapped.
#[derive(Component)]
struct SwapTarget;

#[derive(Component)]
struct Crosshair;

#[derive(Component)]
struct StatusOverlay;

// ---------------------------------------------------------------------------
// Input actions for toggling
// ---------------------------------------------------------------------------

#[derive(InputAction)]
#[action_output(bool)]
struct Toggle1Action;

#[derive(InputAction)]
#[action_output(bool)]
struct Toggle2Action;

#[derive(Component)]
struct AimFocusInputContext;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct ControlState {
    pillar_enabled: bool,
    sphere_is_hold: bool,
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> AppExit {
    let mut app = common::base_app("interaction / aim_focus");

    app.insert_resource(ControlState {
        pillar_enabled: true,
        sphere_is_hold: false,
    });

    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<AimFocusInputContext>();

    app.add_observer(on_toggle_1);
    app.add_observer(on_toggle_2);

    app.add_systems(Startup, setup);
    app.add_systems(Update, (
        toggle_pillar_enabled,
        swap_sphere_behavior,
        update_status_overlay,
        on_completed,
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
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);

    // Player — key config: high alignment_weight so only aimed-at objects focus
    let player = common::spawn_player(&mut commands, Vec3::new(0.0, 1.6, 8.0));
    commands.entity(player).insert((
        saddle_interaction::Interactor {
            max_distance: Some(12.0),
            proximity_radius: Some(12.0),
            // --- AIM-TO-FOCUS TUNING ---
            // High alignment_weight makes the dot(aim, direction) dominate scoring.
            // Objects off-center score so low they lose focus to whatever is centered.
            alignment_weight: 3.0,
            // Low distance_weight means nearby-but-off-screen objects don't steal focus.
            distance_weight: 0.2,
            target_priority_weight: 0.3,
            require_line_of_sight: true,
            ..default()
        },
        AimFocusInputContext,
        actions!(AimFocusInputContext[
            (Action::<Toggle1Action>::new(), bindings![KeyCode::Digit1]),
            (Action::<Toggle2Action>::new(), bindings![KeyCode::Digit2]),
        ]),
    ));

    // Four interactable objects spread around the room
    // 1. Red pillar — can be toggled on/off with key 1
    let slots = vec![InteractionSlot::instant("activate", "Activate Pillar")];
    let pillar = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Red Pillar", common::PropShape::Cylinder { radius: 0.3, height: 2.0 },
        Vec3::new(-4.0, 1.0, 0.0), Color::srgb(0.8, 0.2, 0.15),
    );
    commands.entity(pillar).insert((
        ToggleTarget,
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // 2. Green sphere — slot behavior swaps between instant/hold with key 2
    let slots = vec![InteractionSlot::instant("use", "Use Orb")];
    let sphere = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Green Orb", common::PropShape::Sphere(0.5),
        Vec3::new(4.0, 0.5, 0.0), Color::srgb(0.2, 0.7, 0.3),
    );
    commands.entity(sphere).insert((
        SwapTarget,
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // 3. Blue terminal — always available, reference point
    let slots = vec![InteractionSlot::instant("scan", "Scan Terminal")];
    let terminal = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Blue Terminal", common::PropShape::Cube(Vec3::new(0.8, 1.2, 0.4)),
        Vec3::new(0.0, 0.6, -4.0), Color::srgb(0.15, 0.35, 0.65),
    );
    commands.entity(terminal).insert((
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // 4. Gold crate — always available
    let slots = vec![InteractionSlot::instant("open", "Open Crate")];
    let crate_ent = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Gold Crate", common::PropShape::Cube(Vec3::new(0.7, 0.7, 0.7)),
        Vec3::new(0.0, 0.35, 4.0), Color::srgb(0.75, 0.6, 0.15),
    );
    commands.entity(crate_ent).insert((
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // Center crosshair (small dot)
    commands.spawn((
        Name::new("Crosshair"),
        Crosshair,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(4.0),
            height: Val::Px(4.0),
            margin: UiRect::all(Val::Px(-2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
        GlobalZIndex(20),
    ));

    // Status overlay (top-right)
    commands.spawn((
        Name::new("Status"),
        StatusOverlay,
        Text::new(""),
        TextFont { font_size: 15.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(16.0),
            max_width: Val::Px(320.0),
            ..default()
        },
    ));
}

// ---------------------------------------------------------------------------
// Toggle observers
// ---------------------------------------------------------------------------

fn on_toggle_1(_trigger: On<Start<Toggle1Action>>, mut state: ResMut<ControlState>) {
    state.pillar_enabled = !state.pillar_enabled;
}

fn on_toggle_2(_trigger: On<Start<Toggle2Action>>, mut state: ResMut<ControlState>) {
    state.sphere_is_hold = !state.sphere_is_hold;
}

// ---------------------------------------------------------------------------
// Dynamic control systems
// ---------------------------------------------------------------------------

/// Toggle the red pillar's `Interactable.enabled` at runtime.
fn toggle_pillar_enabled(
    state: Res<ControlState>,
    mut pillars: Query<&mut Interactable, With<ToggleTarget>>,
) {
    if !state.is_changed() {
        return;
    }
    for mut interactable in &mut pillars {
        interactable.enabled = state.pillar_enabled;
    }
}

/// Swap the green sphere's slot between instant and hold at runtime.
fn swap_sphere_behavior(
    state: Res<ControlState>,
    mut spheres: Query<&mut InteractionTarget, With<SwapTarget>>,
) {
    if !state.is_changed() {
        return;
    }
    for mut target in &mut spheres {
        if let Some(slot) = target.slots.first_mut() {
            if state.sphere_is_hold {
                slot.behavior = InteractionBehavior::Single(InteractionExecution::Hold {
                    duration_seconds: 1.5,
                });
                slot.prompt.action_label_key = "Charge Orb".into();
            } else {
                slot.behavior = InteractionBehavior::Single(InteractionExecution::Instant);
                slot.prompt.action_label_key = "Use Orb".into();
            }
        }
    }
}

fn on_completed(
    mut reader: MessageReader<InteractionCompleted>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    targets: Query<&DemoTargetColors>,
) {
    for event in reader.read() {
        let Ok(colors) = targets.get(event.target) else { continue };
        if let Some(mat) = materials.get_mut(&colors.handle) {
            mat.base_color = Color::srgb(0.2, 0.8, 0.4);
        }
    }
}

// ---------------------------------------------------------------------------
// Status overlay
// ---------------------------------------------------------------------------

fn update_status_overlay(
    state: Res<ControlState>,
    mut overlay: Query<&mut Text, With<StatusOverlay>>,
) {
    let Ok(mut text) = overlay.single_mut() else { return };
    **text = format!(
        "AIM-FOCUS DEMO\n\
         Look directly at objects to focus them.\n\
         Only the centered object gets a prompt.\n\n\
         [1] Red Pillar: {}\n\
         [2] Green Orb: {}\n\n\
         WASD: move | Mouse: look | E: interact",
        if state.pillar_enabled { "ENABLED" } else { "DISABLED" },
        if state.sphere_is_hold { "Hold (1.5s)" } else { "Instant" },
    );
}
