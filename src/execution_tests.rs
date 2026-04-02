use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};

use super::*;
use crate::{
    InteractionPlugin,
    components::{
        ActiveInteraction, FocusedInteraction, Interactable, InteractionAvailabilityReason,
        InteractionConsumption, InteractionSlot, InteractionTarget, Interactor,
    },
    messages::InteractionIntent,
};

#[derive(Resource, Default)]
struct TestLog {
    completed: usize,
    canceled: usize,
    failed: usize,
    stage_advanced: usize,
    last_canceled: Option<crate::components::InteractionCancelReason>,
    last_failed: Option<InteractionAvailabilityReason>,
}

fn setup_runtime(slot: InteractionSlot, config: InteractionConfig) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    app.add_plugins(InteractionPlugin::default().with_config(config));
    app.init_resource::<TestLog>();
    app.add_systems(
        PostUpdate,
        (
            |mut log: ResMut<TestLog>,
             mut reader: MessageReader<crate::messages::InteractionCompleted>| {
                log.completed += reader.read().count();
            },
            |mut log: ResMut<TestLog>,
             mut reader: MessageReader<crate::messages::InteractionCanceled>| {
                for event in reader.read() {
                    log.canceled += 1;
                    log.last_canceled = Some(event.reason.clone());
                }
            },
            |mut log: ResMut<TestLog>,
             mut reader: MessageReader<crate::messages::InteractionFailed>| {
                for event in reader.read() {
                    log.failed += 1;
                    log.last_failed = Some(event.reason.clone());
                }
            },
            |mut log: ResMut<TestLog>,
             mut reader: MessageReader<crate::messages::InteractionStageAdvanced>| {
                log.stage_advanced += reader.read().count();
            },
        ),
    );

    let interactor = app
        .world_mut()
        .spawn((
            Name::new("Interactor"),
            Interactor {
                max_distance: Some(5.0),
                proximity_radius: Some(5.0),
                ..default()
            },
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().spawn((
        Name::new("Target"),
        Interactable::default(),
        InteractionTarget { slots: vec![slot] },
        GlobalTransform::from_xyz(1.0, 0.0, 0.0),
    ));

    (app, interactor)
}

#[test]
fn instant_interaction_completes_same_frame() {
    let (mut app, interactor) = setup_runtime(
        InteractionSlot::instant("open", "interaction.open"),
        InteractionConfig::default(),
    );

    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    app.update();

    assert_eq!(app.world().resource::<TestLog>().completed, 1);
    assert!(app.world().get::<ActiveInteraction>(interactor).is_none());
}

#[test]
fn hold_progress_reaches_one_after_duration() {
    let slot = InteractionSlot {
        behavior: crate::components::InteractionBehavior::Single(InteractionExecution::Hold {
            duration_seconds: 0.25,
        }),
        ..InteractionSlot::instant("hold", "interaction.hold")
    };
    let (mut app, interactor) = setup_runtime(slot, InteractionConfig::default());

    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    for _ in 0..4 {
        app.update();
    }

    assert_eq!(app.world().resource::<TestLog>().completed, 1);
}

#[test]
fn hold_cancels_on_release() {
    let slot = InteractionSlot {
        behavior: crate::components::InteractionBehavior::Single(InteractionExecution::Hold {
            duration_seconds: 1.0,
        }),
        ..InteractionSlot::instant("hold", "interaction.hold")
    };
    let (mut app, interactor) = setup_runtime(slot, InteractionConfig::default());

    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    app.update();
    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Release,
    });
    app.update();

    assert_eq!(app.world().resource::<TestLog>().canceled, 1);
}

#[test]
fn hold_cancels_on_explicit_cancel() {
    let slot = InteractionSlot {
        behavior: crate::components::InteractionBehavior::Single(InteractionExecution::Hold {
            duration_seconds: 1.0,
        }),
        ..InteractionSlot::instant("hold", "interaction.hold")
    };
    let (mut app, interactor) = setup_runtime(slot, InteractionConfig::default());

    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    app.update();
    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Cancel,
    });
    app.update();

    let log = app.world().resource::<TestLog>();
    assert_eq!(log.canceled, 1);
    assert_eq!(
        log.last_canceled,
        Some(crate::components::InteractionCancelReason::ExplicitCancel)
    );
}

#[test]
fn chain_advances_until_terminal_stage() {
    let slot = InteractionSlot {
        behavior: crate::components::InteractionBehavior::Sequence {
            stages: vec![
                crate::components::InteractionStage {
                    id: "stage_a".into(),
                    execution: InteractionExecution::Instant,
                    prompt: None,
                },
                crate::components::InteractionStage {
                    id: "stage_b".into(),
                    execution: InteractionExecution::Instant,
                    prompt: None,
                },
            ],
            advance_mode: crate::components::SequenceAdvanceMode::StopAtLast,
        },
        ..InteractionSlot::instant("chain", "interaction.chain")
    };
    let (mut app, interactor) = setup_runtime(slot, InteractionConfig::default());

    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    app.update();
    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    app.update();

    assert_eq!(app.world().resource::<TestLog>().completed, 2);
    assert_eq!(app.world().resource::<TestLog>().stage_advanced, 1);
    let focus = app.world().get::<FocusedInteraction>(interactor).unwrap();
    assert!(focus.slot_id.is_some());
}

#[test]
fn accessibility_mode_transforms_hold_to_toggle() {
    let slot = InteractionSlot {
        behavior: crate::components::InteractionBehavior::Single(InteractionExecution::Hold {
            duration_seconds: 2.0,
        }),
        ..InteractionSlot::instant("hold", "interaction.hold")
    };
    let config = InteractionConfig {
        hold_to_toggle: true,
        ..default()
    };
    let (mut app, interactor) = setup_runtime(slot, config);

    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    app.update();

    assert_eq!(app.world().resource::<TestLog>().completed, 1);
    assert!(app.world().get::<ActiveInteraction>(interactor).is_none());
}

#[test]
fn accessibility_mode_transforms_mash_to_hold() {
    let slot = InteractionSlot {
        behavior: crate::components::InteractionBehavior::Single(InteractionExecution::Mash {
            required_presses: 3,
            decay_per_second: 0.0,
        }),
        ..InteractionSlot::instant("mash", "interaction.mash")
    };
    let config = InteractionConfig {
        mash_auto_complete: true,
        ..default()
    };
    let (mut app, interactor) = setup_runtime(slot, config);

    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    app.update();

    let active = app
        .world()
        .get::<ActiveInteraction>(interactor)
        .expect("mash accessibility should convert into an active hold");
    assert!(matches!(
        active.execution,
        InteractionExecution::Hold { duration_seconds } if duration_seconds > 0.0
    ));

    for _ in 0..6 {
        app.update();
    }

    assert_eq!(app.world().resource::<TestLog>().completed, 1);
    assert!(app.world().get::<ActiveInteraction>(interactor).is_none());
}

#[test]
fn auto_interact_starts_without_manual_confirm() {
    let slot = InteractionSlot {
        auto_trigger_on_focus: true,
        behavior: crate::components::InteractionBehavior::Single(InteractionExecution::Instant),
        ..InteractionSlot::instant("auto", "interaction.auto")
    };
    let (mut app, interactor) = setup_runtime(slot, InteractionConfig::default());

    for _ in 0..6 {
        app.update();
    }

    let focus = app.world().get::<FocusedInteraction>(interactor).unwrap();
    assert!(focus.target.is_some());
    assert_eq!(app.world().resource::<TestLog>().completed, 1);
    assert_eq!(app.world().resource::<TestLog>().failed, 0);
}

#[test]
fn one_shot_consumption_blocks_repeat_attempts() {
    let slot = InteractionSlot {
        availability: crate::components::InteractionAvailabilityConfig {
            consumption: InteractionConsumption::OnceGlobal,
            ..default()
        },
        ..InteractionSlot::instant("use_once", "interaction.use_once")
    };
    let (mut app, interactor) = setup_runtime(slot, InteractionConfig::default());

    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    app.update();
    app.world_mut().write_message(InteractionIntent {
        interactor,
        kind: crate::messages::InteractionIntentKind::Press,
    });
    app.update();

    let log = app.world().resource::<TestLog>();
    assert_eq!(log.completed, 1);
    assert_eq!(log.failed, 1);
    assert_eq!(
        log.last_failed,
        Some(InteractionAvailabilityReason::Consumed)
    );
}
