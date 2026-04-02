use bevy::prelude::*;

use crate::{
    components::{FocusedInteraction, Interactor},
    config::InteractionConfig,
};

pub(crate) fn update_focus(
    config: Res<InteractionConfig>,
    mut interactors: Query<(
        &Interactor,
        &crate::components::InteractionCandidates,
        &mut FocusedInteraction,
    )>,
) {
    for (interactor, candidates, mut focused) in &mut interactors {
        let hysteresis = interactor.hysteresis.unwrap_or(config.hysteresis);
        let best = candidates.entries.first().cloned();

        let Some(best) = best else {
            *focused = FocusedInteraction::default();
            continue;
        };

        let keep_existing = focused
            .target
            .and_then(|target| {
                candidates
                    .entries
                    .iter()
                    .find(|entry| entry.target == target)
                    .map(|entry| entry.score + hysteresis >= best.score)
            })
            .unwrap_or(false);

        if keep_existing {
            if let Some(existing_target) = focused.target {
                if let Some(current) = candidates
                    .entries
                    .iter()
                    .find(|entry| entry.target == existing_target)
                {
                    focused.source = Some(current.source);
                }
            }
            continue;
        }

        focused.target = Some(best.target);
        focused.slot_id = None;
        focused.source = Some(best.source);
    }
}
