use bevy::prelude::*;

use super::*;
use crate::{
    components::{
        FocusSource, FocusedInteraction, Interactable, InteractionCandidate, InteractionCandidates,
        InteractionSlot, InteractionTarget, Interactor,
    },
    config::{InteractionConfig, InteractionStats},
    focus::update_focus,
};

#[test]
fn priority_bonus_can_override_distance() {
    let mut app = App::new();
    app.insert_resource(InteractionConfig::default());
    app.insert_resource(InteractionStats::default());

    let near = app
        .world_mut()
        .spawn((
            Interactable {
                priority: 0.0,
                ..default()
            },
            InteractionTarget {
                slots: vec![InteractionSlot {
                    priority: 0.0,
                    ..InteractionSlot::instant("near", "interaction.near")
                }],
            },
        ))
        .id();
    let far_but_important = app
        .world_mut()
        .spawn((
            Interactable {
                priority: 2.0,
                ..default()
            },
            InteractionTarget {
                slots: vec![InteractionSlot {
                    priority: 2.0,
                    ..InteractionSlot::instant("important", "interaction.important")
                }],
            },
        ))
        .id();

    let interactor = app
        .world_mut()
        .spawn((
            Interactor {
                max_distance: Some(10.0),
                distance_weight: 0.5,
                target_priority_weight: 1.5,
                slot_priority_weight: 1.0,
                ..default()
            },
            InteractionCandidates {
                entries: vec![
                    InteractionCandidate {
                        target: near,
                        source: FocusSource::Proximity,
                        distance: 1.0,
                        ..default()
                    },
                    InteractionCandidate {
                        target: far_but_important,
                        source: FocusSource::Proximity,
                        distance: 4.0,
                        ..default()
                    },
                ],
            },
        ))
        .id();

    app.add_systems(Update, score_candidates);
    app.update();

    let candidates = app
        .world()
        .get::<InteractionCandidates>(interactor)
        .unwrap();
    assert_eq!(candidates.entries[0].target, far_but_important);
}

#[test]
fn sticky_focus_does_not_thrash_between_close_scores() {
    let mut app = App::new();
    app.insert_resource(InteractionConfig {
        hysteresis: 0.3,
        ..default()
    });

    let first = app.world_mut().spawn_empty().id();
    let second = app.world_mut().spawn_empty().id();
    let interactor = app
        .world_mut()
        .spawn((
            Interactor {
                hysteresis: Some(0.3),
                ..default()
            },
            InteractionCandidates {
                entries: vec![
                    InteractionCandidate {
                        target: second,
                        score: 1.0,
                        ..default()
                    },
                    InteractionCandidate {
                        target: first,
                        score: 0.85,
                        ..default()
                    },
                ],
            },
            FocusedInteraction {
                target: Some(first),
                slot_id: None,
                source: Some(FocusSource::Proximity),
            },
        ))
        .id();

    app.add_systems(Update, update_focus);
    app.update();

    let focus = app.world().get::<FocusedInteraction>(interactor).unwrap();
    assert_eq!(focus.target, Some(first));
}
