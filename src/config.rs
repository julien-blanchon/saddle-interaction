use std::collections::HashMap;

use bevy::prelude::*;

use crate::components::{DetectionMode, InteractionPredicateId};

#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionConfig {
    pub detection_mode: DetectionMode,
    pub default_max_distance: f32,
    pub default_proximity_radius: f32,
    pub default_candidate_limit: usize,
    pub hysteresis: f32,
    pub default_input_buffer_seconds: f32,
    pub hold_time_scale: f32,
    pub detection_radius_scale: f32,
    pub hold_to_toggle: bool,
    pub mash_auto_complete: bool,
    pub auto_interact_on_focus: bool,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            detection_mode: DetectionMode::Proximity,
            default_max_distance: 4.0,
            default_proximity_radius: 3.0,
            default_candidate_limit: 8,
            hysteresis: 0.15,
            default_input_buffer_seconds: 0.12,
            hold_time_scale: 1.0,
            detection_radius_scale: 1.0,
            hold_to_toggle: false,
            mash_auto_complete: false,
            auto_interact_on_focus: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct InteractionPredicateFailure {
    pub detail_key: Option<String>,
}

type PredicateFn = dyn Send
    + Sync
    + 'static
    + Fn(&World, Entity, Entity) -> Result<(), InteractionPredicateFailure>;

#[derive(Resource, Default)]
pub struct InteractionPredicateRegistry {
    evaluators: HashMap<InteractionPredicateId, Box<PredicateFn>>,
}

impl InteractionPredicateRegistry {
    pub fn register<F>(&mut self, id: impl Into<InteractionPredicateId>, predicate: F)
    where
        F: Send
            + Sync
            + 'static
            + Fn(&World, Entity, Entity) -> Result<(), InteractionPredicateFailure>,
    {
        self.evaluators.insert(id.into(), Box::new(predicate));
    }

    pub fn evaluate(
        &self,
        id: &InteractionPredicateId,
        world: &World,
        actor: Entity,
        target: Entity,
    ) -> Result<(), InteractionPredicateFailure> {
        self.evaluators
            .get(id)
            .map_or(Ok(()), |predicate| predicate(world, actor, target))
    }
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Reflect)]
pub struct InteractionStats {
    pub interactor_count: usize,
    pub candidate_count: usize,
    pub active_interaction_count: usize,
    pub reservation_count: usize,
}
