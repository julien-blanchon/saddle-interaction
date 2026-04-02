mod components;
mod config;
mod debug;
mod detection;
mod execution;
mod focus;
mod gating;
mod messages;
mod prompt;
mod scoring;
mod util;

pub use components::{
    ActiveInteraction, DetectionMode, FocusSource, FocusedInteraction, Interactable,
    InteractionAvailabilityConfig, InteractionAvailabilityReason, InteractionBehavior,
    InteractionCancelPolicy, InteractionCancelReason, InteractionCandidate, InteractionCandidates,
    InteractionChannel, InteractionConsumption, InteractionCooldown, InteractionExecution,
    InteractionFocusedBy, InteractionOccluder, InteractionOccluderShape, InteractionOffer,
    InteractionPredicateId, InteractionPrompt, InteractionPromptState,
    InteractionReservationPolicy, InteractionSlot, InteractionSlotId, InteractionStage,
    InteractionStageId, InteractionTag, InteractionTags, InteractionTarget, Interactor,
    InteractorAim, InteractorBusyPolicy, InteractorPointer, LineOfSightPolicy,
    PromptSuppressionPolicy, SequenceAdvanceMode,
};
pub use config::{
    InteractionConfig, InteractionPredicateFailure, InteractionPredicateRegistry, InteractionStats,
};
pub use debug::InteractionDebugSettings;
pub use messages::{
    FocusChanged, InteractionCanceled, InteractionCompleted, InteractionExternalCancel,
    InteractionFailed, InteractionIntent, InteractionIntentKind, InteractionOffered,
    InteractionPredicateInvalidated, InteractionProgress, InteractionStageAdvanced,
    InteractionStarted,
};

use bevy::{
    app::PostStartup,
    ecs::{intern::Interned, schedule::ScheduleLabel},
    prelude::*,
};

#[derive(SystemSet, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum InteractionSystems {
    Detect,
    Score,
    Focus,
    Gate,
    Execute,
    Feedback,
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct NeverDeactivateSchedule;

pub struct InteractionPlugin {
    pub activate_schedule: Interned<dyn ScheduleLabel>,
    pub deactivate_schedule: Interned<dyn ScheduleLabel>,
    pub update_schedule: Interned<dyn ScheduleLabel>,
    pub config: InteractionConfig,
}

impl InteractionPlugin {
    pub fn new(
        activate_schedule: impl ScheduleLabel,
        deactivate_schedule: impl ScheduleLabel,
        update_schedule: impl ScheduleLabel,
    ) -> Self {
        Self {
            activate_schedule: activate_schedule.intern(),
            deactivate_schedule: deactivate_schedule.intern(),
            update_schedule: update_schedule.intern(),
            config: InteractionConfig::default(),
        }
    }

    pub fn always_on(update_schedule: impl ScheduleLabel) -> Self {
        Self::new(PostStartup, NeverDeactivateSchedule, update_schedule)
    }

    pub fn with_config(mut self, config: InteractionConfig) -> Self {
        self.config = config;
        self
    }
}

impl Default for InteractionPlugin {
    fn default() -> Self {
        Self::always_on(Update)
    }
}

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        if self.deactivate_schedule == NeverDeactivateSchedule.intern() {
            app.init_schedule(NeverDeactivateSchedule);
        }

        if !app.world().contains_resource::<InteractionConfig>() {
            app.insert_resource(self.config.clone());
        }

        app.init_resource::<InteractionPredicateRegistry>()
            .init_resource::<InteractionStats>()
            .init_resource::<InteractionDebugSettings>()
            .init_resource::<util::InteractionRuntimeState>()
            .init_resource::<util::SpatialHashIndex>()
            .add_message::<InteractionIntent>()
            .add_message::<FocusChanged>()
            .add_message::<InteractionOffered>()
            .add_message::<InteractionStarted>()
            .add_message::<InteractionProgress>()
            .add_message::<InteractionCompleted>()
            .add_message::<InteractionCanceled>()
            .add_message::<InteractionFailed>()
            .add_message::<InteractionStageAdvanced>()
            .add_message::<InteractionExternalCancel>()
            .add_message::<InteractionPredicateInvalidated>()
            .register_type::<ActiveInteraction>()
            .register_type::<DetectionMode>()
            .register_type::<FocusedInteraction>()
            .register_type::<FocusSource>()
            .register_type::<InteractionAvailabilityConfig>()
            .register_type::<InteractionAvailabilityReason>()
            .register_type::<InteractionBehavior>()
            .register_type::<InteractionCandidate>()
            .register_type::<InteractionCandidates>()
            .register_type::<InteractionCancelPolicy>()
            .register_type::<InteractionCancelReason>()
            .register_type::<InteractionChannel>()
            .register_type::<InteractionConfig>()
            .register_type::<InteractionConsumption>()
            .register_type::<InteractionCooldown>()
            .register_type::<InteractionDebugSettings>()
            .register_type::<InteractionExecution>()
            .register_type::<InteractionFocusedBy>()
            .register_type::<InteractionOccluder>()
            .register_type::<InteractionOccluderShape>()
            .register_type::<InteractionOffer>()
            .register_type::<InteractionPrompt>()
            .register_type::<InteractionPromptState>()
            .register_type::<InteractionReservationPolicy>()
            .register_type::<InteractionSlot>()
            .register_type::<InteractionSlotId>()
            .register_type::<InteractionStage>()
            .register_type::<InteractionStageId>()
            .register_type::<InteractionStats>()
            .register_type::<InteractionTag>()
            .register_type::<InteractionTags>()
            .register_type::<InteractionTarget>()
            .register_type::<Interactable>()
            .register_type::<Interactor>()
            .register_type::<InteractorAim>()
            .register_type::<InteractorBusyPolicy>()
            .register_type::<InteractorPointer>()
            .register_type::<LineOfSightPolicy>()
            .register_type::<PromptSuppressionPolicy>()
            .register_type::<SequenceAdvanceMode>()
            .configure_sets(
                self.update_schedule,
                (
                    InteractionSystems::Detect,
                    InteractionSystems::Score,
                    InteractionSystems::Focus,
                    InteractionSystems::Gate,
                    InteractionSystems::Execute,
                    InteractionSystems::Feedback,
                )
                    .chain(),
            )
            .add_systems(self.activate_schedule, detection::activate_runtime)
            .add_systems(self.deactivate_schedule, detection::deactivate_runtime)
            .add_systems(
                self.update_schedule,
                (
                    detection::tick_runtime,
                    detection::prepare_interactors,
                    detection::apply_intents,
                    detection::rebuild_spatial_index,
                    detection::collect_candidates,
                )
                    .chain()
                    .in_set(InteractionSystems::Detect)
                    .run_if(detection::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                scoring::score_candidates
                    .in_set(InteractionSystems::Score)
                    .run_if(detection::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                focus::update_focus
                    .in_set(InteractionSystems::Focus)
                    .run_if(detection::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                gating::evaluate_focus_offer
                    .in_set(InteractionSystems::Gate)
                    .run_if(detection::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                execution::run_interactions
                    .in_set(InteractionSystems::Execute)
                    .run_if(detection::runtime_is_active),
            )
            .add_systems(
                self.update_schedule,
                (
                    prompt::update_focus_markers,
                    prompt::emit_feedback_messages,
                    detection::clear_frame_controls,
                )
                    .chain()
                    .in_set(InteractionSystems::Feedback)
                    .run_if(detection::runtime_is_active),
            )
            .add_systems(
                PostUpdate,
                debug::draw_debug
                    .run_if(detection::runtime_is_active)
                    .run_if(debug::debug_enabled),
            );
    }
}

#[cfg(test)]
#[path = "plugin_tests.rs"]
mod plugin_tests;
