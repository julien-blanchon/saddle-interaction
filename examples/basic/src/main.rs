//! # Basic Interaction Example
//!
//! Walk around with WASD + mouse look. Approach the chest and press **E** to
//! open it. Demonstrates the simplest interaction: a single instant slot.
//!
//! **Concepts**: `Interactor`, `Interactable`, `InteractionTarget`,
//! `InteractionSlot::instant`, `InteractionCompleted`.

use bevy::prelude::*;
use saddle_interaction::{
    Interactable, InteractionCompleted, InteractionSlot, InteractionTarget,
};
use saddle_interaction_example_common as common;
use common::{DemoBaseTargetSlots, DemoTargetColors};

fn main() -> AppExit {
    let mut app = common::base_app("interaction / basic");
    app.add_systems(Startup, setup);
    app.add_systems(Update, on_chest_opened);
    app.run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);
    common::spawn_player(&mut commands, Vec3::new(0.0, 1.6, 5.0));

    // Chest — a single instant "Open" interaction
    let slots = vec![InteractionSlot::instant("open", "Open")];
    let chest = common::spawn_prop(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Chest",
        common::PropShape::Cube(Vec3::new(1.0, 0.8, 0.7)),
        Vec3::new(0.0, 0.4, 0.0),
        Color::srgb(0.55, 0.35, 0.15),
    );
    commands.entity(chest).insert((
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));
}

/// When the chest is opened, change its color to green.
fn on_chest_opened(
    mut reader: MessageReader<InteractionCompleted>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    targets: Query<&DemoTargetColors>,
) {
    for event in reader.read() {
        if event.slot_id.0 == "open" {
            let Ok(colors) = targets.get(event.target) else {
                continue;
            };
            if let Some(mat) = materials.get_mut(&colors.handle) {
                mat.base_color = Color::srgb(0.2, 0.7, 0.3);
            }
        }
    }
}
