use bevy::prelude::*;

use crate::{
    components::{
        FocusedInteraction, Interactable, InteractionAvailabilityReason, InteractionOffer,
        InteractionPromptState, InteractionTags, InteractionTarget, Interactor, LineOfSightPolicy,
        PromptSuppressionPolicy,
    },
    config::{InteractionConfig, InteractionPredicateRegistry},
    detection::InteractorControlState,
    util::{
        InteractionRuntimeState, TargetSlotKey, matches_channel, prompt_for_slot, segment_blocked,
    },
};

pub(crate) fn evaluate_focus_offer(world: &mut World) {
    let config = world.resource::<InteractionConfig>().clone();
    let runtime = world.resource::<InteractionRuntimeState>().clone();

    let interactor_entities: Vec<Entity> = {
        let mut query = world.query_filtered::<Entity, With<Interactor>>();
        query.iter(world).collect()
    };

    for interactor_entity in interactor_entities {
        let Some((interactor, focus, control, interactor_transform, interactor_tags)) = world
            .query::<(
                &Interactor,
                &FocusedInteraction,
                &InteractorControlState,
                &GlobalTransform,
                Option<&InteractionTags>,
            )>()
            .get(world, interactor_entity)
            .ok()
            .map(|(interactor, focus, control, transform, tags)| {
                (
                    interactor.clone(),
                    focus.clone(),
                    control.clone(),
                    *transform,
                    tags.cloned(),
                )
            })
        else {
            continue;
        };

        let Some(target_entity) = focus.target else {
            if let Ok(mut prompt_state) = world
                .query::<&mut InteractionPromptState>()
                .get_mut(world, interactor_entity)
            {
                prompt_state.offer = None;
            }
            if let Ok(mut control) = world
                .query::<&mut InteractorControlState>()
                .get_mut(world, interactor_entity)
            {
                control.auto_started_offer = None;
            }
            if let Ok(mut focused) = world
                .query::<&mut FocusedInteraction>()
                .get_mut(world, interactor_entity)
            {
                focused.slot_id = None;
            }
            continue;
        };

        let Ok((interactable, target, target_transform, target_tags)) = world
            .query::<(
                &Interactable,
                &InteractionTarget,
                &GlobalTransform,
                Option<&InteractionTags>,
            )>()
            .get(world, target_entity)
            .map(|(interactable, target, transform, tags)| {
                (
                    interactable.clone(),
                    target.clone(),
                    *transform,
                    tags.cloned(),
                )
            })
        else {
            if let Ok(mut prompt_state) = world
                .query::<&mut InteractionPromptState>()
                .get_mut(world, interactor_entity)
            {
                prompt_state.offer = None;
            }
            if let Ok(mut control) = world
                .query::<&mut InteractorControlState>()
                .get_mut(world, interactor_entity)
            {
                control.auto_started_offer = None;
            }
            if let Ok(mut focused) = world
                .query::<&mut FocusedInteraction>()
                .get_mut(world, interactor_entity)
            {
                focused.target = None;
                focused.slot_id = None;
                focused.source = None;
            }
            continue;
        };

        if target.slots.is_empty() {
            if let Ok(mut prompt_state) = world
                .query::<&mut InteractionPromptState>()
                .get_mut(world, interactor_entity)
            {
                prompt_state.offer = None;
            }
            if let Ok(mut control) = world
                .query::<&mut InteractorControlState>()
                .get_mut(world, interactor_entity)
            {
                control.auto_started_offer = None;
            }
            continue;
        }

        let mut sorted_indices: Vec<usize> = (0..target.slots.len()).collect();
        sorted_indices.sort_by(|left, right| {
            target.slots[*right]
                .priority
                .total_cmp(&target.slots[*left].priority)
        });

        let previous_slot_index = focus
            .slot_id
            .as_ref()
            .and_then(|selected| target.slots.iter().position(|slot| &slot.id == selected));

        let mut desired_index = previous_slot_index;
        if let Some(selected_slot) = &control.select_slot {
            desired_index = target
                .slots
                .iter()
                .position(|slot| &slot.id == selected_slot);
        } else if control.cycle_delta != 0 {
            let ordered: Vec<usize> = sorted_indices.clone();
            let current_index = desired_index
                .and_then(|selected| ordered.iter().position(|entry| *entry == selected))
                .unwrap_or(0);
            let next_index = (current_index as i32 + control.cycle_delta as i32)
                .rem_euclid(ordered.len() as i32) as usize;
            desired_index = ordered.get(next_index).copied();
        }

        let slot_evaluations: Vec<(usize, Option<InteractionAvailabilityReason>)> = sorted_indices
            .iter()
            .map(|index| {
                (
                    *index,
                    evaluate_slot(
                        world,
                        &config,
                        &runtime,
                        interactor_entity,
                        target_entity,
                        &interactor,
                        &interactable,
                        &target.slots[*index],
                        &interactor_transform,
                        &target_transform,
                        interactor_tags.as_ref(),
                        target_tags.as_ref(),
                    ),
                )
            })
            .collect();

        let chosen_index = desired_index.or_else(|| {
            slot_evaluations
                .iter()
                .find(|(_, availability)| availability.is_none())
                .map(|(index, _)| *index)
                .or_else(|| slot_evaluations.first().map(|(index, _)| *index))
        });

        let Some(chosen_index) = chosen_index else {
            continue;
        };
        let chosen_slot = &target.slots[chosen_index];
        let availability = slot_evaluations
            .iter()
            .find(|(index, _)| *index == chosen_index)
            .and_then(|(_, availability)| availability.clone());
        let (prompt, stage_id, _) = prompt_for_slot(&runtime, target_entity, chosen_slot);
        let suppressed = match prompt.suppression {
            PromptSuppressionPolicy::Never => false,
            PromptSuppressionPolicy::HideWhenUnavailable => availability.is_some(),
            PromptSuppressionPolicy::Always => true,
        };

        if let Ok(mut focused) = world
            .query::<&mut FocusedInteraction>()
            .get_mut(world, interactor_entity)
        {
            focused.slot_id = Some(chosen_slot.id.clone());
        }

        if let Ok(mut prompt_state) = world
            .query::<&mut InteractionPromptState>()
            .get_mut(world, interactor_entity)
        {
            prompt_state.offer = Some(InteractionOffer {
                target: target_entity,
                slot_id: chosen_slot.id.clone(),
                stage_id,
                prompt,
                availability,
                suppressed,
                source: focus.source.unwrap_or_default(),
            });
        }
    }
}

pub(crate) fn evaluate_slot(
    world: &mut World,
    config: &InteractionConfig,
    runtime: &InteractionRuntimeState,
    interactor_entity: Entity,
    target_entity: Entity,
    interactor: &Interactor,
    interactable: &Interactable,
    slot: &crate::components::InteractionSlot,
    interactor_transform: &GlobalTransform,
    target_transform: &GlobalTransform,
    interactor_tags: Option<&InteractionTags>,
    target_tags: Option<&InteractionTags>,
) -> Option<InteractionAvailabilityReason> {
    if !interactable.enabled || !slot.availability.enabled {
        return Some(InteractionAvailabilityReason::Disabled);
    }
    if !matches_channel(&interactor.channels, &interactable.channels) {
        return Some(InteractionAvailabilityReason::Disabled);
    }

    let target_key = TargetSlotKey::new(target_entity, &slot.id);
    if runtime
        .reservations
        .get(&target_key)
        .is_some_and(|owner| *owner != interactor_entity)
        && matches!(
            slot.reservation,
            crate::components::InteractionReservationPolicy::Exclusive
        )
    {
        return Some(InteractionAvailabilityReason::ReservedByOther);
    }
    if runtime.consumed_global.contains(&target_key)
        && matches!(
            slot.availability.consumption,
            crate::components::InteractionConsumption::OnceGlobal
        )
    {
        return Some(InteractionAvailabilityReason::Consumed);
    }
    if runtime
        .consumed_per_actor
        .contains(&(interactor_entity, target_key.clone()))
        && matches!(
            slot.availability.consumption,
            crate::components::InteractionConsumption::OncePerActor
        )
    {
        return Some(InteractionAvailabilityReason::Consumed);
    }

    let time = world.resource::<Time>();
    if let Some(expiry) = runtime.shared_cooldowns.get(&target_key) {
        let remaining = (*expiry - time.elapsed_secs_f64()) as f32;
        if remaining > 0.0 {
            return Some(InteractionAvailabilityReason::SharedCooldown {
                remaining_seconds: remaining,
            });
        }
    }
    if let Some(expiry) = runtime
        .per_actor_cooldowns
        .get(&(interactor_entity, target_key.clone()))
    {
        let remaining = (*expiry - time.elapsed_secs_f64()) as f32;
        if remaining > 0.0 {
            return Some(InteractionAvailabilityReason::PerActorCooldown {
                remaining_seconds: remaining,
            });
        }
    }

    let actor_tags = interactor_tags.cloned().unwrap_or_default();
    for required_tag in &slot.availability.required_actor_tags {
        if !actor_tags.contains(required_tag) {
            return Some(InteractionAvailabilityReason::MissingActorTag(
                required_tag.clone(),
            ));
        }
    }
    for blocked_tag in &slot.availability.blocked_actor_tags {
        if actor_tags.contains(blocked_tag) {
            return Some(InteractionAvailabilityReason::BlockedActorTag(
                blocked_tag.clone(),
            ));
        }
    }

    let target_tags = target_tags.cloned().unwrap_or_default();
    for required_tag in &slot.availability.required_target_tags {
        if !target_tags.contains(required_tag) {
            return Some(InteractionAvailabilityReason::MissingTargetTag(
                required_tag.clone(),
            ));
        }
    }
    for blocked_tag in &slot.availability.blocked_target_tags {
        if target_tags.contains(blocked_tag) {
            return Some(InteractionAvailabilityReason::BlockedTargetTag(
                blocked_tag.clone(),
            ));
        }
    }

    let max_distance = interactor
        .max_distance
        .unwrap_or(config.default_max_distance);
    let target_position = target_transform.translation() + interactable.anchor_offset;
    if interactor_transform.translation().distance(target_position) > max_distance {
        return Some(InteractionAvailabilityReason::OutOfRange);
    }

    let must_have_line_of_sight = match interactable.line_of_sight_policy {
        LineOfSightPolicy::Ignore => false,
        LineOfSightPolicy::UseInteractorSetting => interactor.require_line_of_sight,
        LineOfSightPolicy::Require => true,
    };
    if must_have_line_of_sight
        && segment_blocked(
            world,
            interactor_transform.translation(),
            target_position,
            target_entity,
        )
    {
        return Some(InteractionAvailabilityReason::LineOfSightBlocked);
    }

    let registry = world.resource::<InteractionPredicateRegistry>();
    for predicate in &slot.availability.predicate_ids {
        if let Err(failure) = registry.evaluate(predicate, world, interactor_entity, target_entity)
        {
            return Some(InteractionAvailabilityReason::PredicateFailed {
                predicate: predicate.clone(),
                detail_key: failure.detail_key,
            });
        }
    }

    None
}

#[cfg(test)]
#[path = "gating_tests.rs"]
mod tests;
