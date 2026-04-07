//! # Pickup Prompt — Cross-Crate Example
//!
//! Combines **saddle-physics-object-interaction** (physics-based grab/carry/throw)
//! with **saddle-interaction** (prompt arbitration). Walk up to a prop, see
//! "Press E to Pick Up" via saddle-interaction's focus system, then the physics
//! crate handles the actual grab.
//!
//! **Cross-crate integration points**:
//! - **saddle-interaction** owns detection, focus, prompt, and intent
//! - **saddle-physics-object-interaction** owns the physics hold/release
//! - On `InteractionCompleted("pickup")`, a `TryAcquireObject` message is sent
//! - On `InteractionCompleted("drop")`, a `ReleaseHeldObject` message is sent

use bevy::prelude::*;
use avian3d::prelude::*;
use saddle_interaction::{
    InteractionCompleted, Interactable, InteractionSlot, InteractionTarget,
};
use saddle_physics_object_interaction::{
    AcquisitionConfig, HoldConfig, InteractableBody, ObjectInteractionConfig,
    ObjectInteractionPlugin, ObjectInteractor, ObjectInteractionState,
    TryAcquireObject, ReleaseHeldObject,
};
use saddle_interaction_example_common as common;
use common::DemoBaseTargetSlots;

fn main() -> AppExit {
    let mut app = common::base_app("interaction / pickup_prompt");

    // Physics
    app.insert_resource(Time::<Fixed>::from_hz(60.0));
    app.add_plugins(PhysicsPlugins::default());

    // Object-interaction config: tune acquisition range and hold stability
    app.insert_resource(ObjectInteractionConfig {
        acquisition: AcquisitionConfig {
            max_distance: 5.0,
            ..default()
        },
        hold: HoldConfig {
            // Increase damping to prevent spinning/oscillation
            angular_damping: 24.0,
            linear_damping: 32.0,
            // Reduce max force so objects don't fly away
            max_force: 800.0,
            max_torque: 120.0,
            ..default()
        },
        ..default()
    });
    app.add_plugins(ObjectInteractionPlugin::default());

    app.add_systems(Startup, setup);
    app.add_systems(Update, bridge_interaction_to_physics);

    app.run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);

    // Player — common module gives us movement + interaction; we add ObjectInteractor for physics grab
    let player = common::spawn_player(&mut commands, Vec3::new(0.0, 1.6, 6.0));
    commands.entity(player).insert((
        ObjectInteractor {
            max_target_mass: Some(45.0),
            ..default()
        },
    ));

    // Ground with physics collider (common's ground plane has no collider)
    commands.spawn((
        Name::new("Physics Ground"),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
        Transform::default(),
        Visibility::Hidden,
    ));

    // Pickable props — have both Interactable (prompt) and InteractableBody (physics)
    for (pos, color, label) in [
        (Vec3::new(-2.0, 0.5, 1.0), Color::srgb(0.7, 0.3, 0.2), "Red Box"),
        (Vec3::new(0.0, 0.5, 0.0), Color::srgb(0.2, 0.5, 0.7), "Blue Box"),
        (Vec3::new(2.0, 0.5, 1.0), Color::srgb(0.6, 0.6, 0.2), "Gold Box"),
    ] {
        let slots = vec![InteractionSlot::instant("pickup", "Pick Up")];
        commands.spawn((
            Name::new(label.to_string()),
            // Physics body with damping for stable hold
            RigidBody::Dynamic,
            Collider::cuboid(0.6, 0.6, 0.6),
            LinearDamping(0.4),
            AngularDamping(1.0),
            InteractableBody::default(),
            // Interaction prompt
            DemoBaseTargetSlots(slots.clone()),
            Interactable::default(),
            InteractionTarget { slots },
            // Rendering
            Mesh3d(meshes.add(Cuboid::new(0.6, 0.6, 0.6))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })),
            Transform::from_translation(pos),
        ));
    }
}

/// Bridge: when saddle-interaction completes "pickup", tell the physics crate to grab.
fn bridge_interaction_to_physics(
    mut completed: MessageReader<InteractionCompleted>,
    mut acquire: MessageWriter<TryAcquireObject>,
    mut release: MessageWriter<ReleaseHeldObject>,
    states: Query<&ObjectInteractionState>,
) {
    for event in completed.read() {
        if event.slot_id.0 != "pickup" {
            continue;
        }
        // If already holding, drop instead
        if let Ok(state) = states.get(event.interactor) {
            if matches!(state, ObjectInteractionState::Holding(_)) {
                release.write(ReleaseHeldObject { interactor: event.interactor });
                continue;
            }
        }
        acquire.write(TryAcquireObject { interactor: event.interactor });
    }
}
