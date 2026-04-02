use bevy::prelude::*;

use crate::{
    components::{FocusedInteraction, InteractionFocusedBy, InteractionPromptState, Interactor},
    messages::{FocusChanged, InteractionOffered},
};

#[derive(Component, Clone, Debug, Default)]
pub(crate) struct InteractionFeedbackCache {
    pub focus: Option<FocusedInteraction>,
    pub offer: Option<crate::components::InteractionOffer>,
}

pub(crate) fn update_focus_markers(
    mut commands: Commands,
    mut marked_targets: Query<(Entity, Option<&mut InteractionFocusedBy>)>,
    interactors: Query<(Entity, &FocusedInteraction), With<Interactor>>,
) {
    for (_, focused_by) in &mut marked_targets {
        if let Some(mut focused_by) = focused_by {
            focused_by.interactors.clear();
        }
    }

    for (interactor_entity, focus) in &interactors {
        let Some(target) = focus.target else {
            continue;
        };

        if let Ok((_, Some(mut focused_by))) = marked_targets.get_mut(target) {
            focused_by.interactors.push(interactor_entity);
        } else {
            commands.entity(target).insert(InteractionFocusedBy {
                interactors: vec![interactor_entity],
            });
        }
    }
}

pub(crate) fn emit_feedback_messages(
    mut focus_changed: MessageWriter<FocusChanged>,
    mut offered: MessageWriter<InteractionOffered>,
    mut interactors: Query<
        (
            Entity,
            &FocusedInteraction,
            &InteractionPromptState,
            &mut InteractionFeedbackCache,
        ),
        With<Interactor>,
    >,
) {
    for (entity, focus, prompt, mut cache) in &mut interactors {
        let focus_snapshot = focus.target.is_some().then_some(focus.clone());
        if cache.focus != focus_snapshot {
            focus_changed.write(FocusChanged {
                interactor: entity,
                previous: cache.focus.clone(),
                current: focus_snapshot.clone(),
            });
            cache.focus = focus_snapshot;
        }

        if cache.offer != prompt.offer {
            offered.write(InteractionOffered {
                interactor: entity,
                offer: prompt.offer.clone(),
            });
            cache.offer = prompt.offer.clone();
        }
    }
}
