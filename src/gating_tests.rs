use bevy::prelude::*;

use super::*;
use crate::{
    components::{
        DetectionMode, Interactable, InteractionAvailabilityConfig, InteractionCooldown,
        InteractionSlot, InteractionTag, InteractionTags, InteractionTarget, Interactor,
    },
    config::{InteractionConfig, InteractionPredicateRegistry},
    util::{InteractionRuntimeState, TargetSlotKey},
};

fn setup_world() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(InteractionConfig::default());
    app.insert_resource(InteractionRuntimeState::default());
    app.init_resource::<InteractionPredicateRegistry>();
    app
}

#[test]
fn failed_gate_reports_reason() {
    let mut app = setup_world();
    let actor = app
        .world_mut()
        .spawn((
            Interactor {
                detection_mode: Some(DetectionMode::Proximity),
                ..default()
            },
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
            InteractionTags::default(),
        ))
        .id();
    let target = app
        .world_mut()
        .spawn((
            Interactable::default(),
            InteractionTarget {
                slots: vec![InteractionSlot {
                    availability: InteractionAvailabilityConfig {
                        required_actor_tags: vec![InteractionTag::from("keycard")],
                        ..default()
                    },
                    ..InteractionSlot::instant("use", "interaction.use")
                }],
            },
            GlobalTransform::from_xyz(1.0, 0.0, 0.0),
        ))
        .id();

    let interactor_component = app.world().get::<Interactor>(actor).unwrap().clone();
    let interactable = app.world().get::<Interactable>(target).unwrap().clone();
    let slot = app.world().get::<InteractionTarget>(target).unwrap().slots[0].clone();
    let actor_transform = *app.world().get::<GlobalTransform>(actor).unwrap();
    let target_transform = *app.world().get::<GlobalTransform>(target).unwrap();
    let actor_tags = app.world().get::<InteractionTags>(actor).cloned();
    let config = app.world().resource::<InteractionConfig>().clone();
    let runtime = app.world().resource::<InteractionRuntimeState>().clone();

    let reason = evaluate_slot(
        app.world_mut(),
        &config,
        &runtime,
        actor,
        target,
        &interactor_component,
        &interactable,
        &slot,
        &actor_transform,
        &target_transform,
        actor_tags.as_ref(),
        None,
    );

    assert_eq!(
        reason,
        Some(InteractionAvailabilityReason::MissingActorTag(
            InteractionTag::from("keycard")
        ))
    );
}

#[test]
fn per_actor_cooldown_does_not_block_other_actor() {
    let mut app = setup_world();
    let first_actor = app
        .world_mut()
        .spawn((
            Interactor::default(),
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let second_actor = app
        .world_mut()
        .spawn((
            Interactor::default(),
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let target = app
        .world_mut()
        .spawn((
            Interactable::default(),
            InteractionTarget {
                slots: vec![InteractionSlot {
                    cooldown: InteractionCooldown {
                        per_actor_seconds: 2.0,
                        ..default()
                    },
                    ..InteractionSlot::instant("use", "interaction.use")
                }],
            },
            GlobalTransform::from_xyz(1.0, 0.0, 0.0),
        ))
        .id();

    let slot = app.world().get::<InteractionTarget>(target).unwrap().slots[0].clone();
    let key = TargetSlotKey::new(target, &slot.id);
    app.world_mut()
        .resource_mut::<InteractionRuntimeState>()
        .per_actor_cooldowns
        .insert((first_actor, key), 10.0);

    let interactor_component = app.world().get::<Interactor>(first_actor).unwrap().clone();
    let actor_transform = *app.world().get::<GlobalTransform>(first_actor).unwrap();
    let target_transform = *app.world().get::<GlobalTransform>(target).unwrap();
    let interactable = app.world().get::<Interactable>(target).unwrap().clone();
    let config = app.world().resource::<InteractionConfig>().clone();
    let runtime = app.world().resource::<InteractionRuntimeState>().clone();

    let first_reason = evaluate_slot(
        app.world_mut(),
        &config,
        &runtime,
        first_actor,
        target,
        &interactor_component,
        &interactable,
        &slot,
        &actor_transform,
        &target_transform,
        None,
        None,
    );
    let second_reason = evaluate_slot(
        app.world_mut(),
        &config,
        &runtime,
        second_actor,
        target,
        &interactor_component,
        &interactable,
        &slot,
        &actor_transform,
        &target_transform,
        None,
        None,
    );

    assert!(matches!(
        first_reason,
        Some(InteractionAvailabilityReason::PerActorCooldown { .. })
    ));
    assert_eq!(second_reason, None);
}
