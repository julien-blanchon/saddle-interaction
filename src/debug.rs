use bevy::prelude::*;

use crate::components::{Interactable, InteractionCandidates, Interactor};

#[derive(Resource, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionDebugSettings {
    pub enabled: bool,
    pub draw_proximity_rings: bool,
    pub draw_candidate_lines: bool,
    pub draw_focus_lines: bool,
}

impl Default for InteractionDebugSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            draw_proximity_rings: true,
            draw_candidate_lines: true,
            draw_focus_lines: true,
        }
    }
}

pub(crate) fn debug_enabled(settings: Res<InteractionDebugSettings>) -> bool {
    settings.enabled
}

pub(crate) fn draw_debug(
    settings: Res<InteractionDebugSettings>,
    interactors: Query<(
        &Interactor,
        &GlobalTransform,
        Option<&crate::components::FocusedInteraction>,
        Option<&InteractionCandidates>,
    )>,
    targets: Query<(&Interactable, &GlobalTransform)>,
    mut gizmos: Gizmos,
) {
    if !settings.enabled {
        return;
    }

    for (interactor, transform, focus, candidates) in &interactors {
        if settings.draw_proximity_rings {
            if let Some(radius) = interactor.proximity_radius {
                gizmos.circle(
                    Isometry3d::from_translation(transform.translation()),
                    radius,
                    Color::srgb(0.12, 0.65, 1.0),
                );
            }
        }

        if settings.draw_candidate_lines {
            if let Some(candidates) = candidates {
                for candidate in &candidates.entries {
                    if let Ok((interactable, target_transform)) = targets.get(candidate.target) {
                        gizmos.line(
                            transform.translation(),
                            target_transform.translation() + interactable.anchor_offset,
                            Color::srgba(1.0, 1.0, 1.0, 0.25),
                        );
                    }
                }
            }
        }

        if settings.draw_focus_lines {
            if let Some(focus) = focus.and_then(|focus| focus.target) {
                if let Ok((interactable, target_transform)) = targets.get(focus) {
                    gizmos.line(
                        transform.translation(),
                        target_transform.translation() + interactable.anchor_offset,
                        Color::srgb(1.0, 0.8, 0.15),
                    );
                }
            }
        }
    }
}
