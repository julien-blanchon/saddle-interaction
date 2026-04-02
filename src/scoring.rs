use bevy::prelude::*;

use crate::{
    components::{Interactable, InteractionCandidates, InteractionTarget, Interactor},
    config::{InteractionConfig, InteractionStats},
};

pub(crate) fn score_candidates(
    config: Res<InteractionConfig>,
    mut stats: ResMut<InteractionStats>,
    mut interactors: Query<(&Interactor, &mut InteractionCandidates), With<Interactor>>,
    targets: Query<(&Interactable, &InteractionTarget)>,
) {
    let mut total_candidates = 0;

    for (interactor, mut candidates) in &mut interactors {
        let max_distance = interactor
            .max_distance
            .unwrap_or(config.default_max_distance)
            .max(0.001);
        let limit = interactor
            .candidate_limit
            .unwrap_or(config.default_candidate_limit)
            .max(1);

        for candidate in &mut candidates.entries {
            let distance_score = (1.0 - (candidate.distance / max_distance)).clamp(0.0, 1.0);
            let alignment_score = candidate.alignment.max(0.0);
            let source_bias = match candidate.source {
                crate::components::FocusSource::Picking => interactor.picking_bias,
                crate::components::FocusSource::Hybrid => interactor.picking_bias * 1.15,
                crate::components::FocusSource::Proximity => 0.0,
            };
            let slot_priority = targets
                .get(candidate.target)
                .ok()
                .and_then(|(_, target)| {
                    target
                        .slots
                        .iter()
                        .map(|slot| slot.priority)
                        .max_by(f32::total_cmp)
                })
                .unwrap_or(candidate.slot_priority);

            candidate.score = (distance_score * interactor.distance_weight)
                + (alignment_score * interactor.alignment_weight)
                + (candidate.target_priority * interactor.target_priority_weight)
                + (slot_priority * interactor.slot_priority_weight)
                + source_bias;
        }

        candidates.entries.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.distance.total_cmp(&right.distance))
        });
        candidates.entries.truncate(limit);
        total_candidates += candidates.entries.len();
    }

    stats.candidate_count = total_candidates;
}

#[cfg(test)]
#[path = "scoring_tests.rs"]
mod tests;
