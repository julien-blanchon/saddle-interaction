//! # Vehicle Enter/Exit Example
//!
//! Walk to the vehicle cockpit and press **E** to enter. While "seated", only
//! the exit hatch is available. Press **E** again to exit. Tags and exclusive
//! reservations manage the state.
//!
//! **Concepts**: `InteractionReservationPolicy::Exclusive`,
//! `blocked_actor_tags`, `required_actor_tags`, enter/exit flow.

use bevy::prelude::*;
use saddle_interaction::{
    Interactable, InteractionAvailabilityConfig, InteractionCompleted,
    InteractionReservationPolicy, InteractionSlot, InteractionTag, InteractionTags,
    InteractionTarget,
};
use saddle_interaction_example_common as common;
use common::{DemoBaseTargetSlots, DemoPlayer, DemoPlayerController};

/// Marks the vehicle body for visual reference.
#[derive(Component)]
struct VehicleBody;

const COCKPIT_POS: Vec3 = Vec3::new(0.0, 0.5, -1.5);
const EXIT_POS: Vec3 = Vec3::new(0.0, 0.5, 2.0);
const PLAYER_START: Vec3 = Vec3::new(0.0, 1.6, 6.0);
const SEATED_POS: Vec3 = Vec3::new(0.0, 1.6, 0.0);

fn main() -> AppExit {
    let mut app = common::base_app("interaction / vehicle");
    app.add_systems(Startup, setup);
    app.add_systems(Update, handle_vehicle_events);
    app.run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);
    common::spawn_player(&mut commands, PLAYER_START);

    // Vehicle body (large box, visual only)
    commands.spawn((
        Name::new("Vehicle"),
        VehicleBody,
        Mesh3d(meshes.add(Cuboid::new(2.5, 1.5, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.4, 0.55),
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_translation(Vec3::new(0.0, 0.75, 0.0)),
    ));

    // Cockpit entry point — blocked when already seated
    let enter_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            blocked_actor_tags: vec![InteractionTag::from("seated")],
            ..default()
        },
        reservation: InteractionReservationPolicy::Exclusive,
        ..InteractionSlot::instant("enter", "Enter Vehicle")
    }];
    let cockpit = common::spawn_prop(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Cockpit",
        common::PropShape::Cube(Vec3::splat(0.5)),
        COCKPIT_POS,
        Color::srgb(0.2, 0.6, 0.3),
    );
    commands.entity(cockpit).insert((
        DemoBaseTargetSlots(enter_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: enter_slots },
    ));

    // Exit hatch — requires seated tag
    let exit_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            required_actor_tags: vec![InteractionTag::from("seated")],
            ..default()
        },
        reservation: InteractionReservationPolicy::Exclusive,
        ..InteractionSlot::instant("exit", "Exit Vehicle")
    }];
    let hatch = common::spawn_prop(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Exit Hatch",
        common::PropShape::Cube(Vec3::splat(0.5)),
        EXIT_POS,
        Color::srgb(0.7, 0.3, 0.2),
    );
    commands.entity(hatch).insert((
        DemoBaseTargetSlots(exit_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: exit_slots },
    ));
}

/// Handle enter/exit by toggling the "seated" tag and teleporting the player.
fn handle_vehicle_events(
    mut commands: Commands,
    mut reader: MessageReader<InteractionCompleted>,
    mut players: Query<(&mut Transform, &mut DemoPlayerController, &InteractionTags), With<DemoPlayer>>,
) {
    for event in reader.read() {
        let Ok((mut transform, mut ctrl, tags)) = players.get_mut(event.interactor) else {
            continue;
        };

        match event.slot_id.0.as_str() {
            "enter" => {
                // Add "seated" tag
                let mut new_tags = tags.clone();
                new_tags.tags.push(InteractionTag::from("seated"));
                commands.entity(event.interactor).insert(new_tags);

                // Teleport to seated position
                transform.translation = SEATED_POS;
                ctrl.yaw = std::f32::consts::PI; // Face exit hatch
                ctrl.pitch = 0.0;
            }
            "exit" => {
                // Remove "seated" tag
                let mut new_tags = tags.clone();
                new_tags.tags.retain(|t| t != &InteractionTag::from("seated"));
                commands.entity(event.interactor).insert(new_tags);

                // Teleport back outside
                transform.translation = PLAYER_START;
                ctrl.yaw = 0.0;
                ctrl.pitch = 0.0;
            }
            _ => {}
        }
    }
}
