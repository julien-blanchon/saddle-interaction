//! # Hold Interaction Example
//!
//! Walk to the valve and **hold E** to turn it. A progress bar fills over
//! 1.5 seconds. Release early to cancel. Hold until complete to finish.
//!
//! **Concepts**: `InteractionExecution::Hold`, `ActiveInteraction.progress`,
//! `InteractionCanceled`.

use bevy::prelude::*;
use saddle_interaction::{
    ActiveInteraction, Interactable, InteractionBehavior, InteractionCanceled,
    InteractionCompleted, InteractionExecution, InteractionSlot, InteractionTarget,
};
use saddle_interaction_example_common as common;
use common::{DemoBaseTargetSlots, DemoPlayer, DemoTargetColors};

/// Marker for the progress bar UI element.
#[derive(Component)]
struct ProgressBar;

fn main() -> AppExit {
    let mut app = common::base_app("interaction / hold");
    app.add_systems(Startup, setup);
    app.add_systems(Update, (update_progress_bar, on_valve_turned, on_valve_canceled));
    app.run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);
    common::spawn_player(&mut commands, Vec3::new(0.0, 1.6, 5.0));

    // Valve — hold 1.5 seconds to turn
    let slots = vec![InteractionSlot {
        behavior: InteractionBehavior::Single(InteractionExecution::Hold {
            duration_seconds: 1.5,
        }),
        ..InteractionSlot::instant("turn", "Turn Valve")
    }];
    let valve = common::spawn_prop(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Valve",
        common::PropShape::Cylinder {
            radius: 0.4,
            height: 1.2,
        },
        Vec3::new(0.0, 0.6, 0.0),
        Color::srgb(0.7, 0.2, 0.15),
    );
    commands.entity(valve).insert((
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // Progress bar (UI)
    commands
        .spawn((
            Name::new("Progress Bar Background"),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(80.0),
                left: Val::Percent(35.0),
                width: Val::Percent(30.0),
                height: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        ))
        .with_children(|parent| {
            parent.spawn((
                ProgressBar,
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.9, 0.7, 0.1)),
            ));
        });
}

fn update_progress_bar(
    active: Query<&ActiveInteraction, With<DemoPlayer>>,
    mut bar: Query<&mut Node, With<ProgressBar>>,
) {
    let progress = active.iter().next().map_or(0.0, |a| a.progress);
    if let Ok(mut node) = bar.single_mut() {
        node.width = Val::Percent(progress * 100.0);
    }
}

fn on_valve_turned(
    mut reader: MessageReader<InteractionCompleted>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    targets: Query<&DemoTargetColors>,
) {
    for event in reader.read() {
        if event.slot_id.0 == "turn" {
            let Ok(colors) = targets.get(event.target) else {
                continue;
            };
            if let Some(mat) = materials.get_mut(&colors.handle) {
                mat.base_color = Color::srgb(0.2, 0.7, 0.3);
            }
        }
    }
}

fn on_valve_canceled(
    mut reader: MessageReader<InteractionCanceled>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    targets: Query<&DemoTargetColors>,
) {
    for event in reader.read() {
        let Ok(colors) = targets.get(event.target) else {
            continue;
        };
        if let Some(mat) = materials.get_mut(&colors.handle) {
            mat.base_color = colors.base;
        }
    }
}
