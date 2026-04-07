//! # Sequence Interaction Example
//!
//! Walk to the lever and press **E** to advance through three stages:
//! **Prime → Pull → Reset**. The lever rotates with each stage. After
//! "Reset" the sequence loops back to "Prime".
//!
//! **Concepts**: `InteractionBehavior::Sequence`, `InteractionStage`,
//! `SequenceAdvanceMode::Loop`, `InteractionStageAdvanced`.

use bevy::prelude::*;
use saddle_interaction::{
    Interactable, InteractionBehavior, InteractionExecution, InteractionPrompt,
    InteractionSlot, InteractionStage, InteractionStageAdvanced, InteractionTarget,
    SequenceAdvanceMode,
};
use saddle_interaction_example_common as common;
use common::DemoBaseTargetSlots;

/// Tracks the current stage index for visual feedback.
#[derive(Component)]
struct LeverState {
    stage: usize,
}

fn main() -> AppExit {
    let mut app = common::base_app("interaction / sequence");
    app.add_systems(Startup, setup);
    app.add_systems(Update, on_stage_advanced);
    app.run()
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);
    common::spawn_player(&mut commands, Vec3::new(0.0, 1.6, 5.0));

    // Lever with a 3-stage sequence
    let slots = vec![InteractionSlot {
        behavior: InteractionBehavior::Sequence {
            stages: vec![
                InteractionStage {
                    id: "prime".into(),
                    execution: InteractionExecution::Instant,
                    prompt: Some(InteractionPrompt {
                        action_label_key: "Prime".into(),
                        ..default()
                    }),
                },
                InteractionStage {
                    id: "pull".into(),
                    execution: InteractionExecution::Instant,
                    prompt: Some(InteractionPrompt {
                        action_label_key: "Pull".into(),
                        ..default()
                    }),
                },
                InteractionStage {
                    id: "reset".into(),
                    execution: InteractionExecution::Instant,
                    prompt: Some(InteractionPrompt {
                        action_label_key: "Reset".into(),
                        ..default()
                    }),
                },
            ],
            advance_mode: SequenceAdvanceMode::Loop,
        },
        ..InteractionSlot::instant("lever", "Prime")
    }];

    let lever = common::spawn_prop(
        &mut commands,
        &mut meshes,
        &mut materials,
        "Lever",
        common::PropShape::Cylinder {
            radius: 0.15,
            height: 1.6,
        },
        Vec3::new(0.0, 0.8, 0.0),
        Color::srgb(0.5, 0.5, 0.55),
    );
    commands.entity(lever).insert((
        LeverState { stage: 0 },
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));
}

/// Rotate the lever on each stage advancement.
fn on_stage_advanced(
    mut reader: MessageReader<InteractionStageAdvanced>,
    mut levers: Query<(&mut Transform, &mut LeverState)>,
) {
    for event in reader.read() {
        let Ok((mut transform, mut state)) = levers.get_mut(event.target) else {
            continue;
        };

        // Advance visual state
        if event.terminal {
            // Looping back to start
            state.stage = 0;
        } else {
            state.stage += 1;
        }

        // Rotate by 45° per stage
        let angle = state.stage as f32 * std::f32::consts::FRAC_PI_4;
        transform.rotation = Quat::from_rotation_z(angle);
    }
}
