use bevy::prelude::*;

use crate::{
    components::{
        ActiveInteraction, FocusedInteraction, Interactable, InteractionAvailabilityReason,
        InteractionBehavior, InteractionCancelReason, InteractionExecution,
        InteractionReservationPolicy, InteractionSlot, InteractionTags, InteractionTarget,
        Interactor,
    },
    config::InteractionConfig,
    gating::evaluate_slot,
    messages::{
        InteractionCanceled, InteractionCompleted, InteractionFailed, InteractionProgress,
        InteractionStageAdvanced, InteractionStarted,
    },
    util::{InteractionRuntimeState, TargetSlotKey, execution_for_slot},
};

#[derive(Default)]
struct BufferedEvents {
    started: Vec<InteractionStarted>,
    progressed: Vec<InteractionProgress>,
    completed: Vec<InteractionCompleted>,
    canceled: Vec<InteractionCanceled>,
    failed: Vec<InteractionFailed>,
    stage_advanced: Vec<InteractionStageAdvanced>,
}

pub(crate) fn run_interactions(world: &mut World) {
    let config = world.resource::<InteractionConfig>().clone();
    let time = world.resource::<Time>().elapsed_secs_f64();
    let interactor_entities: Vec<Entity> = {
        let mut query = world.query_filtered::<Entity, With<Interactor>>();
        query.iter(world).collect()
    };

    let mut events = BufferedEvents::default();

    for interactor_entity in interactor_entities {
        let active_snapshot = world
            .query::<Option<&ActiveInteraction>>()
            .get(world, interactor_entity)
            .ok()
            .flatten()
            .cloned();

        if let Some(active) = active_snapshot {
            if let Some(reason) =
                cancel_active_if_needed(world, &config, interactor_entity, &active)
            {
                release_reservation(world, interactor_entity, active.target, &active.slot_id);
                world
                    .entity_mut(interactor_entity)
                    .remove::<ActiveInteraction>();
                events.canceled.push(InteractionCanceled {
                    interactor: interactor_entity,
                    target: active.target,
                    slot_id: active.slot_id.clone(),
                    reason,
                });
                continue;
            }

            if tick_active(world, &config, interactor_entity, active, time, &mut events) {
                continue;
            }
        }

        try_start(world, &config, interactor_entity, time, &mut events);
    }

    world
        .resource_mut::<InteractionRuntimeState>()
        .pending_external_cancels
        .clear();

    {
        let mut messages = world.resource_mut::<Messages<InteractionStarted>>();
        for event in events.started {
            messages.write(event);
        }
    }
    {
        let mut messages = world.resource_mut::<Messages<InteractionProgress>>();
        for event in events.progressed {
            messages.write(event);
        }
    }
    {
        let mut messages = world.resource_mut::<Messages<InteractionCompleted>>();
        for event in events.completed {
            messages.write(event);
        }
    }
    {
        let mut messages = world.resource_mut::<Messages<InteractionCanceled>>();
        for event in events.canceled {
            messages.write(event);
        }
    }
    {
        let mut messages = world.resource_mut::<Messages<InteractionFailed>>();
        for event in events.failed {
            messages.write(event);
        }
    }
    {
        let mut messages = world.resource_mut::<Messages<InteractionStageAdvanced>>();
        for event in events.stage_advanced {
            messages.write(event);
        }
    }

    let active_count = {
        let mut query = world.query::<&ActiveInteraction>();
        query.iter(world).count()
    };
    let reservation_count = world
        .resource::<InteractionRuntimeState>()
        .reservations
        .len();
    let mut stats = world.resource_mut::<crate::config::InteractionStats>();
    stats.active_interaction_count = active_count;
    stats.reservation_count = reservation_count;
}

fn try_start(
    world: &mut World,
    config: &InteractionConfig,
    interactor_entity: Entity,
    time: f64,
    events: &mut BufferedEvents,
) {
    let Some((focus, prompt_state, interactor, control)) = world
        .query::<(
            &FocusedInteraction,
            &crate::components::InteractionPromptState,
            &Interactor,
            &crate::detection::InteractorControlState,
        )>()
        .get(world, interactor_entity)
        .ok()
        .map(|(focus, prompt, interactor, control)| {
            (
                focus.clone(),
                prompt.clone(),
                interactor.clone(),
                control.clone(),
            )
        })
    else {
        return;
    };

    let Some(target_entity) = focus.target else {
        return;
    };
    let Some(offer) = prompt_state.offer else {
        return;
    };

    let target_snapshot = world
        .query::<(
            &InteractionTarget,
            &Interactable,
            &GlobalTransform,
            Option<&InteractionTags>,
        )>()
        .get(world, target_entity)
        .ok()
        .map(|(target, interactable, transform, tags)| {
            (
                target.clone(),
                interactable.clone(),
                *transform,
                tags.cloned(),
            )
        });
    let Some((target, interactable, target_transform, target_tags)) = target_snapshot else {
        return;
    };
    let Some(slot) = target
        .slots
        .iter()
        .find(|slot| slot.id == offer.slot_id)
        .cloned()
    else {
        return;
    };

    let auto_offer = crate::detection::AutoStartedOffer {
        target: offer.target,
        slot_id: offer.slot_id.clone(),
        stage_id: offer.stage_id.clone(),
    };
    let auto_start_requested = offer.availability.is_none()
        && (config.auto_interact_on_focus || slot.auto_trigger_on_focus)
        && control.auto_started_offer.as_ref() != Some(&auto_offer);
    let wants_start =
        control.confirm_pressed || control.confirm_buffer_remaining > 0.0 || auto_start_requested;
    if !wants_start {
        return;
    }

    let Some((interactor_transform, interactor_tags)) = world
        .query::<(&GlobalTransform, Option<&InteractionTags>)>()
        .get(world, interactor_entity)
        .ok()
        .map(|(transform, tags)| (*transform, tags.cloned()))
    else {
        return;
    };

    let runtime_snapshot = world.resource::<InteractionRuntimeState>().clone();
    if let Some(reason) = evaluate_slot(
        world,
        config,
        &runtime_snapshot,
        interactor_entity,
        target_entity,
        &interactor,
        &interactable,
        &slot,
        &interactor_transform,
        &target_transform,
        interactor_tags.as_ref(),
        target_tags.as_ref(),
    ) {
        events.failed.push(InteractionFailed {
            interactor: interactor_entity,
            target: Some(target_entity),
            slot_id: Some(slot.id.clone()),
            reason,
        });
        return;
    }

    if auto_start_requested
        && let Ok(mut control) = world
            .query::<&mut crate::detection::InteractorControlState>()
            .get_mut(world, interactor_entity)
    {
        control.auto_started_offer = Some(auto_offer);
    }

    let (mut execution, stage_id, stage_index) =
        execution_for_slot(&runtime_snapshot, target_entity, &slot);
    execution = effective_execution(config, execution);

    events.started.push(InteractionStarted {
        interactor: interactor_entity,
        target: target_entity,
        slot_id: slot.id.clone(),
        stage_id: stage_id.clone(),
    });

    if matches!(slot.reservation, InteractionReservationPolicy::Exclusive) {
        world
            .resource_mut::<InteractionRuntimeState>()
            .reservations
            .insert(
                TargetSlotKey::new(target_entity, &slot.id),
                interactor_entity,
            );
    }

    match execution {
        InteractionExecution::Instant => {
            apply_completion(
                world,
                interactor_entity,
                target_entity,
                slot,
                stage_id,
                stage_index,
                None,
                time,
                events,
            );
        }
        InteractionExecution::Toggle => {
            let key = TargetSlotKey::new(target_entity, &slot.id);
            let toggle_state = {
                let mut runtime = world.resource_mut::<InteractionRuntimeState>();
                let next = !runtime.toggle_states.get(&key).copied().unwrap_or(false);
                runtime.toggle_states.insert(key, next);
                next
            };
            apply_completion(
                world,
                interactor_entity,
                target_entity,
                slot,
                stage_id,
                stage_index,
                Some(toggle_state),
                time,
                events,
            );
        }
        InteractionExecution::Hold { .. }
        | InteractionExecution::Mash { .. }
        | InteractionExecution::Passive { .. } => {
            world
                .entity_mut(interactor_entity)
                .insert(ActiveInteraction {
                    target: target_entity,
                    slot_id: slot.id.clone(),
                    stage_id,
                    execution,
                    progress: 0.0,
                    started_at_seconds: time,
                    toggle_state: None,
                    stage_index,
                });
            if let Ok(mut control) = world
                .query::<&mut crate::detection::InteractorControlState>()
                .get_mut(world, interactor_entity)
            {
                control.confirm_buffer_remaining = 0.0;
            }
        }
    }
}

fn tick_active(
    world: &mut World,
    config: &InteractionConfig,
    interactor_entity: Entity,
    active: ActiveInteraction,
    time: f64,
    events: &mut BufferedEvents,
) -> bool {
    let delta = world.resource::<Time>().delta_secs();
    let Some((target, _interactable)) = world
        .query::<(&InteractionTarget, &Interactable)>()
        .get(world, active.target)
        .ok()
        .map(|(target, interactable)| (target.clone(), interactable.clone()))
    else {
        world
            .entity_mut(interactor_entity)
            .remove::<ActiveInteraction>();
        return true;
    };
    let Some(slot) = target
        .slots
        .iter()
        .find(|slot| slot.id == active.slot_id)
        .cloned()
    else {
        world
            .entity_mut(interactor_entity)
            .remove::<ActiveInteraction>();
        return true;
    };

    let control = world
        .query::<&crate::detection::InteractorControlState>()
        .get(world, interactor_entity)
        .ok()
        .cloned()
        .unwrap_or_default();

    let mut next = active.clone();
    match active.execution {
        InteractionExecution::Hold { duration_seconds } => {
            next.progress = (next.progress
                + delta / (duration_seconds * config.hold_time_scale).max(0.01))
            .clamp(0.0, 1.0);
        }
        InteractionExecution::Mash {
            required_presses,
            decay_per_second,
        } => {
            if control.confirm_pressed {
                next.progress += 1.0 / required_presses.max(1) as f32;
            } else if decay_per_second > 0.0 {
                next.progress -= decay_per_second * delta / required_presses.max(1) as f32;
            }
            next.progress = next.progress.clamp(0.0, 1.0);
        }
        InteractionExecution::Passive { duration_seconds } => {
            next.progress = (next.progress + delta / duration_seconds.max(0.01)).clamp(0.0, 1.0);
        }
        InteractionExecution::Instant | InteractionExecution::Toggle => {}
    }

    events.progressed.push(InteractionProgress {
        interactor: interactor_entity,
        target: next.target,
        slot_id: next.slot_id.clone(),
        stage_id: next.stage_id.clone(),
        progress: next.progress,
    });

    if next.progress >= 1.0 {
        apply_completion(
            world,
            interactor_entity,
            next.target,
            slot,
            next.stage_id.clone(),
            next.stage_index,
            next.toggle_state,
            time,
            events,
        );
        world
            .entity_mut(interactor_entity)
            .remove::<ActiveInteraction>();
        return true;
    }

    world.entity_mut(interactor_entity).insert(next);
    true
}

fn cancel_active_if_needed(
    world: &mut World,
    config: &InteractionConfig,
    interactor_entity: Entity,
    active: &ActiveInteraction,
) -> Option<InteractionCancelReason> {
    let runtime = world.resource::<InteractionRuntimeState>().clone();
    if let Some(reason) = runtime.pending_external_cancels.get(&interactor_entity) {
        return Some(reason.clone());
    }

    let focus = world
        .query::<&FocusedInteraction>()
        .get(world, interactor_entity)
        .ok()
        .cloned()
        .unwrap_or_default();
    let control = world
        .query::<&crate::detection::InteractorControlState>()
        .get(world, interactor_entity)
        .ok()
        .cloned()
        .unwrap_or_default();

    let Some((target, interactable, target_transform, target_tags)) = world
        .query::<(
            &InteractionTarget,
            &Interactable,
            &GlobalTransform,
            Option<&InteractionTags>,
        )>()
        .get(world, active.target)
        .ok()
        .map(|(target, interactable, transform, tags)| {
            (
                target.clone(),
                interactable.clone(),
                *transform,
                tags.cloned(),
            )
        })
    else {
        return Some(InteractionCancelReason::TargetMissing);
    };
    let Some((interactor, interactor_transform, interactor_tags)) = world
        .query::<(&Interactor, &GlobalTransform, Option<&InteractionTags>)>()
        .get(world, interactor_entity)
        .ok()
        .map(|(interactor, transform, tags)| (interactor.clone(), *transform, tags.cloned()))
    else {
        return Some(InteractionCancelReason::TargetMissing);
    };

    let slot = target.slots.iter().find(|slot| slot.id == active.slot_id)?;
    if control.cancel_requested {
        return Some(InteractionCancelReason::ExplicitCancel);
    }
    if slot.cancellation.on_release
        && matches!(active.execution, InteractionExecution::Hold { .. })
        && control.confirm_released
    {
        return Some(InteractionCancelReason::InputReleased);
    }
    if slot.cancellation.on_focus_loss
        && (focus.target != Some(active.target) || focus.slot_id.as_ref() != Some(&active.slot_id))
    {
        return Some(InteractionCancelReason::FocusLost);
    }

    if slot.cancellation.on_blocked_state {
        if let Some(reason) = evaluate_slot(
            world,
            config,
            &runtime,
            interactor_entity,
            active.target,
            &interactor,
            &interactable,
            slot,
            &interactor_transform,
            &target_transform,
            interactor_tags.as_ref(),
            target_tags.as_ref(),
        ) {
            return Some(match reason {
                InteractionAvailabilityReason::LineOfSightBlocked => {
                    InteractionCancelReason::LineOfSightBreak
                }
                InteractionAvailabilityReason::OutOfRange => InteractionCancelReason::DistanceBreak,
                InteractionAvailabilityReason::PredicateFailed {
                    predicate,
                    detail_key,
                } => InteractionCancelReason::PredicateInvalidated {
                    predicate,
                    detail_key,
                },
                _ => InteractionCancelReason::Busy,
            });
        }
    }

    None
}

fn apply_completion(
    world: &mut World,
    interactor_entity: Entity,
    target_entity: Entity,
    slot: InteractionSlot,
    stage_id: Option<crate::components::InteractionStageId>,
    stage_index: usize,
    toggle_state: Option<bool>,
    time: f64,
    events: &mut BufferedEvents,
) {
    events.completed.push(InteractionCompleted {
        interactor: interactor_entity,
        target: target_entity,
        slot_id: slot.id.clone(),
        stage_id: stage_id.clone(),
        toggle_state,
    });

    let key = TargetSlotKey::new(target_entity, &slot.id);
    let mut runtime = world.resource_mut::<InteractionRuntimeState>();
    if slot.cooldown.shared_seconds > 0.0 {
        runtime
            .shared_cooldowns
            .insert(key.clone(), time + f64::from(slot.cooldown.shared_seconds));
    }
    if slot.cooldown.per_actor_seconds > 0.0 {
        runtime.per_actor_cooldowns.insert(
            (interactor_entity, key.clone()),
            time + f64::from(slot.cooldown.per_actor_seconds),
        );
    }

    if matches!(
        slot.availability.consumption,
        crate::components::InteractionConsumption::OnceGlobal
    ) {
        runtime.consumed_global.insert(key.clone());
    }
    if matches!(
        slot.availability.consumption,
        crate::components::InteractionConsumption::OncePerActor
    ) {
        runtime
            .consumed_per_actor
            .insert((interactor_entity, key.clone()));
    }

    if let InteractionBehavior::Sequence {
        stages,
        advance_mode,
    } = &slot.behavior
    {
        if !stages.is_empty() {
            let next_index = match advance_mode {
                crate::components::SequenceAdvanceMode::StopAtLast => {
                    (stage_index + 1).min(stages.len().saturating_sub(1))
                }
                crate::components::SequenceAdvanceMode::Loop => (stage_index + 1) % stages.len(),
            };
            runtime.stage_indices.insert(key.clone(), next_index);
            if next_index != stage_index {
                events.stage_advanced.push(InteractionStageAdvanced {
                    interactor: interactor_entity,
                    target: target_entity,
                    slot_id: slot.id.clone(),
                    previous_stage_id: stage_id,
                    next_stage_id: Some(stages[next_index].id.clone()),
                    terminal: matches!(
                        advance_mode,
                        crate::components::SequenceAdvanceMode::StopAtLast
                    ) && next_index == stages.len().saturating_sub(1),
                });
            }
        }
    }

    runtime.reservations.remove(&key);
}

fn effective_execution(
    config: &InteractionConfig,
    execution: InteractionExecution,
) -> InteractionExecution {
    match execution {
        InteractionExecution::Hold { .. } if config.hold_to_toggle => InteractionExecution::Toggle,
        InteractionExecution::Mash {
            required_presses, ..
        } if config.mash_auto_complete => InteractionExecution::Hold {
            duration_seconds: (required_presses as f32 * 0.18).max(0.18),
        },
        other => other,
    }
}

fn release_reservation(
    world: &mut World,
    interactor_entity: Entity,
    target_entity: Entity,
    slot_id: &crate::components::InteractionSlotId,
) {
    let key = TargetSlotKey::new(target_entity, slot_id);
    if world
        .resource::<InteractionRuntimeState>()
        .reservations
        .get(&key)
        .is_some_and(|owner| *owner == interactor_entity)
    {
        world
            .resource_mut::<InteractionRuntimeState>()
            .reservations
            .remove(&key);
    }
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
