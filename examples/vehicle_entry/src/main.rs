//! # Vehicle Entry — Cross-Crate Example
//!
//! Combines **saddle-vehicle-ground-vehicle** with **saddle-interaction** for
//! enter/exit prompts. Walk to the vehicle, press **E** to enter. While
//! "seated", only the exit hatch is available. Press **E** again to exit.
//!
//! **Cross-crate integration points**:
//! - saddle-interaction handles the enter/exit prompt and tag gating
//! - `InteractionReservationPolicy::Exclusive` prevents re-entry while seated
//! - Tags ("seated") manage the state transition

use bevy::prelude::*;
use avian3d::prelude::*;
use saddle_interaction::{
    Interactable, InteractionAvailabilityConfig, InteractionCompleted,
    InteractionReservationPolicy, InteractionSlot, InteractionTag, InteractionTags,
    InteractionTarget,
};
use saddle_vehicle_ground_vehicle::GroundVehiclePlugin;
use saddle_interaction_example_common as common;
use common::{DemoBaseTargetSlots, DemoPlayer, DemoPlayerController};

const PLAYER_START: Vec3 = Vec3::new(0.0, 1.6, 8.0);
const SEATED_POS: Vec3 = Vec3::new(0.0, 1.6, 0.0);

fn main() -> AppExit {
    let mut app = common::base_app("interaction / vehicle_entry");

    // Add physics and vehicle plugin on top of the common base
    app.insert_resource(Time::<Fixed>::from_hz(60.0));
    app.add_plugins(PhysicsPlugins::default());
    app.add_plugins(GroundVehiclePlugin::default());

    app.add_systems(Startup, setup);
    app.add_systems(Update, handle_enter_exit);

    app.run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);
    common::spawn_player(&mut commands, PLAYER_START);

    // Physics ground collider
    commands.spawn((
        Name::new("Physics Ground"),
        RigidBody::Static,
        Collider::half_space(Vec3::Y),
        Transform::default(),
        Visibility::Hidden,
    ));

    // Vehicle body (visual)
    commands.spawn((
        Name::new("Vehicle"),
        Mesh3d(meshes.add(Cuboid::new(2.5, 1.5, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.4, 0.55),
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.75, 0.0),
    ));

    // Entry point — blocked when seated
    let enter_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            blocked_actor_tags: vec![InteractionTag::from("seated")],
            ..default()
        },
        reservation: InteractionReservationPolicy::Exclusive,
        ..InteractionSlot::instant("enter", "Enter Vehicle")
    }];
    let entry = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Entry Point", common::PropShape::Cube(Vec3::splat(0.5)),
        Vec3::new(0.0, 0.5, -2.0), Color::srgb(0.2, 0.6, 0.3),
    );
    commands.entity(entry).insert((
        DemoBaseTargetSlots(enter_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: enter_slots },
    ));

    // Exit point — requires seated
    let exit_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            required_actor_tags: vec![InteractionTag::from("seated")],
            ..default()
        },
        reservation: InteractionReservationPolicy::Exclusive,
        ..InteractionSlot::instant("exit", "Exit Vehicle")
    }];
    let exit = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Exit Point", common::PropShape::Cube(Vec3::splat(0.5)),
        Vec3::new(0.0, 0.5, 2.5), Color::srgb(0.7, 0.3, 0.2),
    );
    commands.entity(exit).insert((
        DemoBaseTargetSlots(exit_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: exit_slots },
    ));
}

/// Handle enter/exit by toggling the "seated" tag and teleporting the player.
fn handle_enter_exit(
    mut commands: Commands,
    mut reader: MessageReader<InteractionCompleted>,
    mut players: Query<(&mut Transform, &mut DemoPlayerController, &InteractionTags), With<DemoPlayer>>,
) {
    for event in reader.read() {
        let Ok((mut transform, mut ctrl, tags)) = players.get_mut(event.interactor) else {
            continue;
        };
        let seated = InteractionTag::from("seated");

        match event.slot_id.0.as_str() {
            "enter" => {
                let mut new_tags = tags.clone();
                new_tags.tags.push(seated);
                commands.entity(event.interactor).insert(new_tags);
                transform.translation = SEATED_POS;
                ctrl.yaw = std::f32::consts::PI;
                ctrl.pitch = 0.0;
            }
            "exit" => {
                let mut new_tags = tags.clone();
                new_tags.tags.retain(|t| t != &seated);
                commands.entity(event.interactor).insert(new_tags);
                transform.translation = PLAYER_START;
                ctrl.yaw = 0.0;
                ctrl.pitch = 0.0;
            }
            _ => {}
        }
    }
}
