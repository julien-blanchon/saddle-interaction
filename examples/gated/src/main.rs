//! # Gated Interaction Example
//!
//! Walk between two objects: a **Generator** (always available) and a
//! **Security Door** (requires the `"powered"` tag). Activate the generator
//! first to gain the tag, then approach the door to unlock it.
//!
//! **Concepts**: `InteractionTags`, `InteractionAvailabilityConfig`,
//! `required_actor_tags`, tag-gated availability.

use bevy::prelude::*;
use saddle_interaction::{
    Interactable, InteractionAvailabilityConfig, InteractionCompleted, InteractionSlot,
    InteractionTag, InteractionTags, InteractionTarget,
};
use saddle_interaction_example_common as common;
use common::{DemoBaseTargetSlots, DemoPlayer, DemoTargetColors};

fn main() -> AppExit {
    let mut app = common::base_app("interaction / gated");
    app.add_systems(Startup, setup);
    app.add_systems(Update, (on_generator_activated, on_door_unlocked));
    app.run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);
    common::spawn_player(&mut commands, Vec3::new(0.0, 1.6, 7.0));

    // Generator — always available, grants "powered" tag on completion
    let gen_slots = vec![InteractionSlot::instant("activate", "Activate Generator")];
    let generator = common::spawn_prop(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Generator",
        common::PropShape::Sphere(0.6),
        Vec3::new(-3.0, 0.6, 0.0),
        Color::srgb(0.8, 0.45, 0.1),
    );
    commands.entity(generator).insert((
        DemoBaseTargetSlots(gen_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: gen_slots },
    ));

    // Security Door — requires "powered" tag on the actor
    let door_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            required_actor_tags: vec![InteractionTag::from("powered")],
            ..default()
        },
        ..InteractionSlot::instant("unlock", "Unlock Door")
    }];
    let door = common::spawn_prop(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Security Door",
        common::PropShape::Cube(Vec3::new(2.0, 2.5, 0.3)),
        Vec3::new(3.0, 1.25, 0.0),
        Color::srgb(0.4, 0.12, 0.12),
    );
    commands.entity(door).insert((
        DemoBaseTargetSlots(door_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: door_slots },
    ));
}

/// When the generator is activated, add the "powered" tag to the player.
fn on_generator_activated(
    mut commands: Commands,
    mut reader: MessageReader<InteractionCompleted>,
    interactors: Query<&InteractionTags, With<DemoPlayer>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    targets: Query<&DemoTargetColors>,
) {
    for event in reader.read() {
        if event.slot_id.0 != "activate" {
            continue;
        }
        // Add "powered" tag
        let mut tags = interactors
            .get(event.interactor)
            .cloned()
            .unwrap_or_default();
        if !tags.contains(&InteractionTag::from("powered")) {
            tags.tags.push(InteractionTag::from("powered"));
        }
        commands.entity(event.interactor).insert(tags);

        // Visual feedback
        let Ok(colors) = targets.get(event.target) else {
            continue;
        };
        if let Some(mat) = materials.get_mut(&colors.handle) {
            mat.base_color = Color::srgb(0.2, 0.8, 0.3);
        }
    }
}

/// When the door is unlocked, change its color.
fn on_door_unlocked(
    mut reader: MessageReader<InteractionCompleted>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    targets: Query<&DemoTargetColors>,
) {
    for event in reader.read() {
        if event.slot_id.0 != "unlock" {
            continue;
        }
        let Ok(colors) = targets.get(event.target) else {
            continue;
        };
        if let Some(mat) = materials.get_mut(&colors.handle) {
            mat.base_color = Color::srgb(0.2, 0.7, 0.3);
        }
    }
}
