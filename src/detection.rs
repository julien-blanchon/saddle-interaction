use bevy::{ecs::hierarchy::ChildOf, picking::pointer::PointerInteraction, prelude::*};

use crate::{
    components::{
        DetectionMode, FocusSource, Interactable, InteractionCandidate, InteractionCandidates,
        InteractionSlotId, InteractionStageId, InteractionTarget, Interactor, InteractorAim,
        InteractorPointer,
    },
    config::{InteractionConfig, InteractionStats},
    messages::{InteractionExternalCancel, InteractionIntent, InteractionIntentKind},
    util::{InteractionRuntimeState, SpatialHashIndex, effective_detection_mode, matches_channel},
};

#[derive(Component, Clone, Debug, Default)]
pub(crate) struct InteractorControlState {
    pub confirm_held: bool,
    pub confirm_pressed: bool,
    pub confirm_released: bool,
    pub cancel_requested: bool,
    pub cycle_delta: i8,
    pub select_slot: Option<crate::components::InteractionSlotId>,
    pub confirm_buffer_remaining: f32,
    pub auto_started_offer: Option<AutoStartedOffer>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AutoStartedOffer {
    pub target: Entity,
    pub slot_id: InteractionSlotId,
    pub stage_id: Option<InteractionStageId>,
}

pub(crate) fn activate_runtime(mut runtime: ResMut<InteractionRuntimeState>) {
    runtime.active = true;
}

pub(crate) fn deactivate_runtime(
    mut commands: Commands,
    mut runtime: ResMut<InteractionRuntimeState>,
    interactors: Query<Entity, With<Interactor>>,
    mut controls: Query<&mut InteractorControlState, With<Interactor>>,
) {
    runtime.active = false;
    runtime.shared_cooldowns.clear();
    runtime.per_actor_cooldowns.clear();
    runtime.consumed_global.clear();
    runtime.consumed_per_actor.clear();
    runtime.reservations.clear();
    runtime.toggle_states.clear();
    runtime.stage_indices.clear();
    runtime.pending_external_cancels.clear();

    for entity in &interactors {
        if let Ok(mut control) = controls.get_mut(entity) {
            *control = InteractorControlState::default();
        }
        commands
            .entity(entity)
            .insert(crate::components::InteractionCandidates::default())
            .insert(crate::components::FocusedInteraction::default())
            .insert(crate::components::InteractionPromptState::default())
            .remove::<crate::components::ActiveInteraction>();
    }
}

pub(crate) fn runtime_is_active(runtime: Res<InteractionRuntimeState>) -> bool {
    runtime.active
}

pub(crate) fn tick_runtime(
    time: Res<Time>,
    config: Res<InteractionConfig>,
    mut stats: ResMut<InteractionStats>,
    mut controls: Query<&mut InteractorControlState, With<Interactor>>,
) {
    stats.interactor_count = 0;
    stats.candidate_count = 0;
    stats.active_interaction_count = 0;

    for mut control in &mut controls {
        if control.confirm_buffer_remaining > 0.0 {
            control.confirm_buffer_remaining =
                (control.confirm_buffer_remaining - time.delta_secs()).max(0.0);
        }

        if control.confirm_pressed {
            control.confirm_buffer_remaining = config.default_input_buffer_seconds;
        }
    }
}

pub(crate) fn prepare_interactors(
    mut commands: Commands,
    interactors: Query<
        Entity,
        (
            With<Interactor>,
            Or<(
                Without<InteractorControlState>,
                Without<InteractionCandidates>,
                Without<crate::components::FocusedInteraction>,
                Without<crate::components::InteractionPromptState>,
                Without<crate::prompt::InteractionFeedbackCache>,
            )>,
        ),
    >,
) {
    for entity in &interactors {
        commands
            .entity(entity)
            .insert(InteractorControlState::default())
            .insert(InteractionCandidates::default())
            .insert(crate::components::FocusedInteraction::default())
            .insert(crate::components::InteractionPromptState::default())
            .insert(crate::prompt::InteractionFeedbackCache::default());
    }
}

pub(crate) fn apply_intents(
    mut intents: MessageReader<InteractionIntent>,
    mut cancels: MessageReader<InteractionExternalCancel>,
    mut controls: Query<&mut InteractorControlState, With<Interactor>>,
    mut runtime: ResMut<InteractionRuntimeState>,
) {
    for intent in intents.read() {
        let Ok(mut control) = controls.get_mut(intent.interactor) else {
            continue;
        };

        match &intent.kind {
            InteractionIntentKind::Press => {
                control.confirm_held = true;
                control.confirm_pressed = true;
            }
            InteractionIntentKind::Release => {
                control.confirm_held = false;
                control.confirm_released = true;
            }
            InteractionIntentKind::Cancel => {
                control.cancel_requested = true;
            }
            InteractionIntentKind::CycleNext => {
                control.cycle_delta = control.cycle_delta.saturating_add(1);
            }
            InteractionIntentKind::CyclePrevious => {
                control.cycle_delta = control.cycle_delta.saturating_sub(1);
            }
            InteractionIntentKind::SelectSlot(slot_id) => {
                control.select_slot = Some(slot_id.clone());
            }
        }
    }

    for cancel in cancels.read() {
        runtime
            .pending_external_cancels
            .insert(cancel.interactor, cancel.reason.clone());
    }
}

pub(crate) fn rebuild_spatial_index(
    config: Res<InteractionConfig>,
    mut index: ResMut<SpatialHashIndex>,
    targets: Query<(Entity, &Interactable, &GlobalTransform), With<InteractionTarget>>,
) {
    index.cells.clear();
    index.cell_size = config.default_proximity_radius.max(0.5) * config.detection_radius_scale;
    let cell_size = index.cell_size.max(0.5);

    for (entity, interactable, transform) in &targets {
        let world_position = transform.translation() + interactable.anchor_offset;
        let cell = IVec3::new(
            (world_position.x / cell_size).floor() as i32,
            (world_position.y / cell_size).floor() as i32,
            (world_position.z / cell_size).floor() as i32,
        );
        index.cells.entry(cell).or_default().push(entity);
    }
}

pub(crate) fn collect_candidates(
    config: Res<InteractionConfig>,
    mut stats: ResMut<InteractionStats>,
    runtime: Res<InteractionRuntimeState>,
    index: Res<SpatialHashIndex>,
    child_of: Query<&ChildOf>,
    pointer_interactions: Query<&PointerInteraction>,
    mut interactors: Query<
        (
            Entity,
            &Interactor,
            Option<&InteractorAim>,
            Option<&InteractorPointer>,
            &GlobalTransform,
            &mut InteractionCandidates,
        ),
        With<Interactor>,
    >,
    targets: Query<(Entity, &Interactable, &InteractionTarget, &GlobalTransform)>,
) {
    if !runtime.active {
        return;
    }

    for (entity, interactor, aim, pointer, transform, mut candidates) in &mut interactors {
        stats.interactor_count += 1;
        candidates.entries.clear();

        let mode = effective_detection_mode(interactor.detection_mode, config.detection_mode);
        let max_distance = interactor
            .max_distance
            .unwrap_or(config.default_max_distance);
        let proximity_radius = interactor
            .proximity_radius
            .unwrap_or(config.default_proximity_radius)
            * config.detection_radius_scale;
        let origin = transform.translation();
        let aim_direction =
            crate::util::resolve_aim_direction(aim.map(|entry| entry.direction), transform);

        if matches!(mode, DetectionMode::Proximity | DetectionMode::Hybrid) {
            let cell_radius = (proximity_radius / index.cell_size.max(0.5)).ceil() as i32;
            let origin_cell = IVec3::new(
                (origin.x / index.cell_size.max(0.5)).floor() as i32,
                (origin.y / index.cell_size.max(0.5)).floor() as i32,
                (origin.z / index.cell_size.max(0.5)).floor() as i32,
            );

            for x in -cell_radius..=cell_radius {
                for y in -cell_radius..=cell_radius {
                    for z in -cell_radius..=cell_radius {
                        let cell = origin_cell + IVec3::new(x, y, z);
                        let Some(entities) = index.cells.get(&cell) else {
                            continue;
                        };

                        for target_entity in entities {
                            let Ok((target_entity, interactable, target, target_transform)) =
                                targets.get(*target_entity)
                            else {
                                continue;
                            };
                            if entity == target_entity
                                || !interactable.enabled
                                || target.slots.is_empty()
                            {
                                continue;
                            }
                            if !matches_channel(&interactor.channels, &interactable.channels) {
                                continue;
                            }

                            let target_position =
                                target_transform.translation() + interactable.anchor_offset;
                            let direction = target_position - origin;
                            let distance = direction.length();
                            let radius = interactable.focus_radius.unwrap_or(proximity_radius);
                            if distance > max_distance || distance > radius {
                                continue;
                            }

                            let alignment = aim_direction.map_or(0.0, |aim_dir| {
                                let direction = direction.normalize_or_zero();
                                aim_dir.dot(direction)
                            });
                            let slot_priority = target
                                .slots
                                .iter()
                                .map(|slot| slot.priority)
                                .fold(f32::NEG_INFINITY, f32::max)
                                .max(0.0);
                            add_or_merge_candidate(
                                &mut candidates.entries,
                                InteractionCandidate {
                                    target: target_entity,
                                    source: FocusSource::Proximity,
                                    distance,
                                    alignment,
                                    slot_priority,
                                    target_priority: interactable.priority,
                                    score: 0.0,
                                },
                            );
                        }
                    }
                }
            }
        }

        if matches!(mode, DetectionMode::Picking | DetectionMode::Hybrid) {
            for pointer_interaction in &pointer_interactions {
                let Some((hit_entity, hit)) = pointer_interaction.get_nearest_hit() else {
                    continue;
                };
                if let Some(camera) = pointer.and_then(|entry| entry.camera)
                    && hit.camera != camera
                {
                    continue;
                }

                let Some(target_entity) =
                    resolve_interactable_entity(*hit_entity, &targets, &child_of)
                else {
                    continue;
                };

                let Ok((target_entity, interactable, target, target_transform)) =
                    targets.get(target_entity)
                else {
                    continue;
                };
                if !interactable.enabled
                    || target.slots.is_empty()
                    || !matches_channel(&interactor.channels, &interactable.channels)
                {
                    continue;
                }

                let target_position = target_transform.translation() + interactable.anchor_offset;
                let distance = origin.distance(target_position);
                if distance > max_distance {
                    continue;
                }

                let slot_priority = target
                    .slots
                    .iter()
                    .map(|slot| slot.priority)
                    .fold(f32::NEG_INFINITY, f32::max)
                    .max(0.0);
                add_or_merge_candidate(
                    &mut candidates.entries,
                    InteractionCandidate {
                        target: target_entity,
                        source: if matches!(mode, DetectionMode::Hybrid) {
                            FocusSource::Hybrid
                        } else {
                            FocusSource::Picking
                        },
                        distance,
                        alignment: 1.0,
                        slot_priority,
                        target_priority: interactable.priority,
                        score: 0.0,
                    },
                );
            }
        }

        stats.candidate_count += candidates.entries.len();
    }
}

pub(crate) fn clear_frame_controls(
    mut controls: Query<&mut InteractorControlState, With<Interactor>>,
) {
    for mut control in &mut controls {
        control.confirm_pressed = false;
        control.confirm_released = false;
        control.cancel_requested = false;
        control.cycle_delta = 0;
        control.select_slot = None;
    }
}

fn add_or_merge_candidate(
    candidates: &mut Vec<InteractionCandidate>,
    candidate: InteractionCandidate,
) {
    if let Some(existing) = candidates
        .iter_mut()
        .find(|entry| entry.target == candidate.target)
    {
        existing.distance = existing.distance.min(candidate.distance);
        existing.alignment = existing.alignment.max(candidate.alignment);
        existing.slot_priority = existing.slot_priority.max(candidate.slot_priority);
        existing.target_priority = existing.target_priority.max(candidate.target_priority);
        existing.source = match (existing.source, candidate.source) {
            (FocusSource::Picking, FocusSource::Proximity)
            | (FocusSource::Proximity, FocusSource::Picking)
            | (_, FocusSource::Hybrid)
            | (FocusSource::Hybrid, _) => FocusSource::Hybrid,
            (current, _) => current,
        };
        return;
    }

    candidates.push(candidate);
}

fn resolve_interactable_entity(
    mut entity: Entity,
    interactables: &Query<(Entity, &Interactable, &InteractionTarget, &GlobalTransform)>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    if interactables.get(entity).is_ok() {
        return Some(entity);
    }

    loop {
        let Ok(parent) = parents.get(entity) else {
            return None;
        };
        entity = parent.parent();
        if interactables.get(entity).is_ok() {
            return Some(entity);
        }
    }
}

#[cfg(test)]
#[path = "detection_tests.rs"]
mod tests;
