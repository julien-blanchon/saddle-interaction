//! # Multi-Slot Interaction Example
//!
//! Walk to the terminal and press **E** to execute the default action. Press
//! **Tab** / **Q** to cycle between "Hack", "Read", and "Override" slots.
//!
//! **Concepts**: multiple `InteractionSlot`s on one target, slot `priority`,
//! `InteractionIntentKind::CycleNext` / `CyclePrevious`.

use bevy::prelude::*;
use saddle_interaction::{
    Interactable, InteractionCompleted, InteractionSlot, InteractionTarget,
};
use saddle_interaction_example_common as common;
use common::{DemoBaseTargetSlots, DemoTargetColors};

fn main() -> AppExit {
    let mut app = common::base_app("interaction / multi_slot");
    app.add_systems(Startup, setup);
    app.add_systems(Update, on_action_completed);
    app.run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);
    common::spawn_player(&mut commands, Vec3::new(0.0, 1.6, 5.0));

    // Terminal with three slots at different priorities
    let slots = vec![
        InteractionSlot {
            priority: 1.1,
            ..InteractionSlot::instant("hack", "Hack")
        },
        InteractionSlot {
            priority: 0.5,
            ..InteractionSlot::instant("read", "Read")
        },
        InteractionSlot {
            priority: 0.1,
            ..InteractionSlot::instant("override", "Override")
        },
    ];
    let terminal = common::spawn_prop(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Terminal",
        common::PropShape::Cube(Vec3::new(0.8, 1.4, 0.4)),
        Vec3::new(0.0, 0.7, 0.0),
        Color::srgb(0.15, 0.35, 0.65),
    );
    commands.entity(terminal).insert((
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));
}

/// Flash the terminal a different color per action.
fn on_action_completed(
    mut reader: MessageReader<InteractionCompleted>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    targets: Query<&DemoTargetColors>,
) {
    for event in reader.read() {
        let Ok(colors) = targets.get(event.target) else {
            continue;
        };
        let Some(mat) = materials.get_mut(&colors.handle) else {
            continue;
        };
        mat.base_color = match event.slot_id.0.as_str() {
            "hack" => Color::srgb(0.9, 0.2, 0.2),
            "read" => Color::srgb(0.2, 0.7, 0.9),
            "override" => Color::srgb(0.9, 0.6, 0.1),
            _ => colors.base,
        };
    }
}
