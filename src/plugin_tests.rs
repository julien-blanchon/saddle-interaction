use bevy::{ecs::schedule::ScheduleLabel, prelude::*};

use crate::{
    InteractionPlugin,
    components::{Interactable, InteractionSlot, InteractionTarget, Interactor},
};

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct TestActivate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct TestDeactivate;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct TestUpdate;

#[test]
fn plugin_supports_custom_update_schedule() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_schedule(TestUpdate);
    app.add_plugins(InteractionPlugin::always_on(TestUpdate));

    let interactor = app
        .world_mut()
        .spawn((
            Interactor {
                max_distance: Some(5.0),
                proximity_radius: Some(5.0),
                ..default()
            },
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().spawn((
        Interactable::default(),
        InteractionTarget {
            slots: vec![InteractionSlot::instant("open", "interaction.open")],
        },
        GlobalTransform::from_xyz(1.0, 0.0, 0.0),
    ));

    app.update();
    assert!(
        app.world()
            .get::<crate::components::InteractionCandidates>(interactor)
            .is_none()
    );

    app.world_mut().run_schedule(TestUpdate);

    assert!(
        !app.world()
            .get::<crate::components::InteractionCandidates>(interactor)
            .unwrap()
            .entries
            .is_empty()
    );
}

#[test]
fn deactivate_schedule_cleans_transient_state() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_schedule(TestActivate);
    app.init_schedule(TestDeactivate);
    app.init_schedule(TestUpdate);
    app.add_plugins(InteractionPlugin::new(
        TestActivate,
        TestDeactivate,
        TestUpdate,
    ));

    let interactor = app
        .world_mut()
        .spawn((
            Interactor {
                max_distance: Some(5.0),
                proximity_radius: Some(5.0),
                ..default()
            },
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().spawn((
        Interactable::default(),
        InteractionTarget {
            slots: vec![InteractionSlot {
                behavior: crate::InteractionBehavior::Single(crate::InteractionExecution::Hold {
                    duration_seconds: 1.0,
                }),
                ..InteractionSlot::instant("hold", "interaction.hold")
            }],
        },
        GlobalTransform::from_xyz(1.0, 0.0, 0.0),
    ));

    app.world_mut().run_schedule(TestActivate);
    app.world_mut().run_schedule(TestUpdate);
    app.world_mut().write_message(crate::InteractionIntent {
        interactor,
        kind: crate::InteractionIntentKind::Press,
    });
    app.world_mut().run_schedule(TestUpdate);

    assert!(
        app.world()
            .get::<crate::ActiveInteraction>(interactor)
            .is_some()
    );

    app.world_mut().run_schedule(TestDeactivate);

    let focus = app
        .world()
        .get::<crate::FocusedInteraction>(interactor)
        .expect("focus component should remain attached");
    assert_eq!(*focus, crate::FocusedInteraction::default());

    let prompt = app
        .world()
        .get::<crate::InteractionPromptState>(interactor)
        .expect("prompt state should remain attached");
    assert!(prompt.offer.is_none());
    assert!(
        app.world()
            .get::<crate::ActiveInteraction>(interactor)
            .is_none()
    );
}
