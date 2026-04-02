use bevy::prelude::*;

use crate::components::{
    FocusedInteraction, InteractionAvailabilityReason, InteractionCancelReason, InteractionOffer,
    InteractionPredicateId, InteractionSlotId, InteractionStageId,
};

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum InteractionIntentKind {
    Press,
    Release,
    Cancel,
    CycleNext,
    CyclePrevious,
    SelectSlot(InteractionSlotId),
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionIntent {
    pub interactor: Entity,
    pub kind: InteractionIntentKind,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct FocusChanged {
    pub interactor: Entity,
    pub previous: Option<FocusedInteraction>,
    pub current: Option<FocusedInteraction>,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionOffered {
    pub interactor: Entity,
    pub offer: Option<InteractionOffer>,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionStarted {
    pub interactor: Entity,
    pub target: Entity,
    pub slot_id: InteractionSlotId,
    pub stage_id: Option<InteractionStageId>,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionProgress {
    pub interactor: Entity,
    pub target: Entity,
    pub slot_id: InteractionSlotId,
    pub stage_id: Option<InteractionStageId>,
    pub progress: f32,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionCompleted {
    pub interactor: Entity,
    pub target: Entity,
    pub slot_id: InteractionSlotId,
    pub stage_id: Option<InteractionStageId>,
    pub toggle_state: Option<bool>,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionCanceled {
    pub interactor: Entity,
    pub target: Entity,
    pub slot_id: InteractionSlotId,
    pub reason: InteractionCancelReason,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionFailed {
    pub interactor: Entity,
    pub target: Option<Entity>,
    pub slot_id: Option<InteractionSlotId>,
    pub reason: InteractionAvailabilityReason,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionStageAdvanced {
    pub interactor: Entity,
    pub target: Entity,
    pub slot_id: InteractionSlotId,
    pub previous_stage_id: Option<InteractionStageId>,
    pub next_stage_id: Option<InteractionStageId>,
    pub terminal: bool,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionExternalCancel {
    pub interactor: Entity,
    pub reason: InteractionCancelReason,
}

#[derive(Message, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionPredicateInvalidated {
    pub interactor: Entity,
    pub predicate: InteractionPredicateId,
}
