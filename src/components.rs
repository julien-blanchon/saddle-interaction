use bevy::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Clone, PartialEq, Hash)]
pub struct InteractionChannel(pub String);

impl InteractionChannel {
    pub fn world() -> Self {
        Self("world".to_owned())
    }
}

impl Default for InteractionChannel {
    fn default() -> Self {
        Self::world()
    }
}

impl From<&str> for InteractionChannel {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Clone, PartialEq, Hash)]
pub struct InteractionTag(pub String);

impl From<&str> for InteractionTag {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Clone, PartialEq, Hash)]
pub struct InteractionPredicateId(pub String);

impl From<&str> for InteractionPredicateId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Clone, PartialEq, Hash)]
pub struct InteractionSlotId(pub String);

impl From<&str> for InteractionSlotId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Clone, PartialEq, Hash)]
pub struct InteractionStageId(pub String);

impl From<&str> for InteractionStageId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum DetectionMode {
    Picking,
    #[default]
    Proximity,
    Hybrid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum FocusSource {
    Picking,
    #[default]
    Proximity,
    Hybrid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum InteractorBusyPolicy {
    #[default]
    SingleActive,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum PromptSuppressionPolicy {
    #[default]
    Never,
    HideWhenUnavailable,
    Always,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum SequenceAdvanceMode {
    #[default]
    StopAtLast,
    Loop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum InteractionReservationPolicy {
    #[default]
    Shared,
    Exclusive,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum InteractionConsumption {
    #[default]
    Reusable,
    OnceGlobal,
    OncePerActor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum LineOfSightPolicy {
    Ignore,
    #[default]
    UseInteractorSetting,
    Require,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Clone, PartialEq)]
pub struct InteractionPrompt {
    pub prompt_key: String,
    pub action_label_key: String,
    pub input_hint_key: Option<String>,
    pub icon_key: Option<String>,
    pub priority: i32,
    pub suppression: PromptSuppressionPolicy,
    pub anchor_entity: Option<Entity>,
    pub anchor_offset: Vec3,
}

impl Default for InteractionPrompt {
    fn default() -> Self {
        Self {
            prompt_key: "interaction.prompt".to_owned(),
            action_label_key: "interaction.action".to_owned(),
            input_hint_key: None,
            icon_key: None,
            priority: 0,
            suppression: PromptSuppressionPolicy::Never,
            anchor_entity: None,
            anchor_offset: Vec3::ZERO,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct InteractionCooldown {
    pub shared_seconds: f32,
    pub per_actor_seconds: f32,
    pub acceptance_delay_seconds: f32,
}

impl Default for InteractionCooldown {
    fn default() -> Self {
        Self {
            shared_seconds: 0.0,
            per_actor_seconds: 0.0,
            acceptance_delay_seconds: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct InteractionAvailabilityConfig {
    pub enabled: bool,
    pub required_actor_tags: Vec<InteractionTag>,
    pub blocked_actor_tags: Vec<InteractionTag>,
    pub required_target_tags: Vec<InteractionTag>,
    pub blocked_target_tags: Vec<InteractionTag>,
    pub predicate_ids: Vec<InteractionPredicateId>,
    pub consumption: InteractionConsumption,
}

impl Default for InteractionAvailabilityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            required_actor_tags: Vec::new(),
            blocked_actor_tags: Vec::new(),
            required_target_tags: Vec::new(),
            blocked_target_tags: Vec::new(),
            predicate_ids: Vec::new(),
            consumption: InteractionConsumption::Reusable,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub struct InteractionCancelPolicy {
    pub on_release: bool,
    pub on_focus_loss: bool,
    pub on_distance_break: bool,
    pub on_line_of_sight_break: bool,
    pub on_blocked_state: bool,
}

impl Default for InteractionCancelPolicy {
    fn default() -> Self {
        Self {
            on_release: true,
            on_focus_loss: true,
            on_distance_break: true,
            on_line_of_sight_break: true,
            on_blocked_state: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub enum InteractionExecution {
    #[default]
    Instant,
    Hold {
        duration_seconds: f32,
    },
    Toggle,
    Mash {
        required_presses: u32,
        decay_per_second: f32,
    },
    Passive {
        duration_seconds: f32,
    },
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct InteractionStage {
    pub id: InteractionStageId,
    pub execution: InteractionExecution,
    pub prompt: Option<InteractionPrompt>,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum InteractionBehavior {
    Single(InteractionExecution),
    Sequence {
        stages: Vec<InteractionStage>,
        advance_mode: SequenceAdvanceMode,
    },
}

impl Default for InteractionBehavior {
    fn default() -> Self {
        Self::Single(InteractionExecution::Instant)
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct InteractionSlot {
    pub id: InteractionSlotId,
    pub prompt: InteractionPrompt,
    pub behavior: InteractionBehavior,
    pub availability: InteractionAvailabilityConfig,
    pub priority: f32,
    pub auto_trigger_on_focus: bool,
    pub cooldown: InteractionCooldown,
    pub cancellation: InteractionCancelPolicy,
    pub reservation: InteractionReservationPolicy,
}

impl InteractionSlot {
    pub fn instant(id: impl Into<InteractionSlotId>, label_key: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            prompt: InteractionPrompt {
                action_label_key: label_key.into(),
                ..default()
            },
            ..default()
        }
    }
}

impl Default for InteractionSlot {
    fn default() -> Self {
        Self {
            id: InteractionSlotId("default".to_owned()),
            prompt: InteractionPrompt::default(),
            behavior: InteractionBehavior::default(),
            availability: InteractionAvailabilityConfig::default(),
            priority: 0.0,
            auto_trigger_on_focus: false,
            cooldown: InteractionCooldown::default(),
            cancellation: InteractionCancelPolicy::default(),
            reservation: InteractionReservationPolicy::default(),
        }
    }
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
pub struct Interactor {
    pub detection_mode: Option<DetectionMode>,
    pub max_distance: Option<f32>,
    pub proximity_radius: Option<f32>,
    pub candidate_limit: Option<usize>,
    pub hysteresis: Option<f32>,
    pub distance_weight: f32,
    pub alignment_weight: f32,
    pub target_priority_weight: f32,
    pub slot_priority_weight: f32,
    pub picking_bias: f32,
    pub require_line_of_sight: bool,
    pub channels: Vec<InteractionChannel>,
    pub busy_policy: InteractorBusyPolicy,
}

impl Default for Interactor {
    fn default() -> Self {
        Self {
            detection_mode: None,
            max_distance: None,
            proximity_radius: None,
            candidate_limit: None,
            hysteresis: None,
            distance_weight: 1.0,
            alignment_weight: 0.35,
            target_priority_weight: 1.0,
            slot_priority_weight: 0.45,
            picking_bias: 0.75,
            require_line_of_sight: false,
            channels: vec![InteractionChannel::world()],
            busy_policy: InteractorBusyPolicy::SingleActive,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub struct InteractorAim {
    pub direction: Vec3,
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
pub struct InteractorPointer {
    pub camera: Option<Entity>,
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
pub struct Interactable {
    pub enabled: bool,
    pub focus_radius: Option<f32>,
    pub priority: f32,
    pub channels: Vec<InteractionChannel>,
    pub line_of_sight_policy: LineOfSightPolicy,
    pub anchor_entity: Option<Entity>,
    pub anchor_offset: Vec3,
}

impl Default for Interactable {
    fn default() -> Self {
        Self {
            enabled: true,
            focus_radius: None,
            priority: 0.0,
            channels: vec![InteractionChannel::world()],
            line_of_sight_policy: LineOfSightPolicy::UseInteractorSetting,
            anchor_entity: None,
            anchor_offset: Vec3::ZERO,
        }
    }
}

#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
pub struct InteractionTarget {
    pub slots: Vec<InteractionSlot>,
}

#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
pub struct InteractionTags {
    pub tags: Vec<InteractionTag>,
}

impl InteractionTags {
    pub fn contains(&self, tag: &InteractionTag) -> bool {
        self.tags.iter().any(|entry| entry == tag)
    }
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum InteractionAvailabilityReason {
    Disabled,
    Busy,
    ReservedByOther,
    Consumed,
    MissingActorTag(InteractionTag),
    BlockedActorTag(InteractionTag),
    MissingTargetTag(InteractionTag),
    BlockedTargetTag(InteractionTag),
    SharedCooldown {
        remaining_seconds: f32,
    },
    PerActorCooldown {
        remaining_seconds: f32,
    },
    PredicateFailed {
        predicate: InteractionPredicateId,
        detail_key: Option<String>,
    },
    OutOfRange,
    LineOfSightBlocked,
    MissingTarget,
    NoSlots,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum InteractionCancelReason {
    ExplicitCancel,
    InputReleased,
    FocusLost,
    DistanceBreak,
    LineOfSightBreak,
    Busy,
    TargetMissing,
    ReservationLost,
    PredicateInvalidated {
        predicate: InteractionPredicateId,
        detail_key: Option<String>,
    },
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct InteractionCandidate {
    pub target: Entity,
    pub source: FocusSource,
    pub distance: f32,
    pub alignment: f32,
    pub slot_priority: f32,
    pub target_priority: f32,
    pub score: f32,
}

impl Default for InteractionCandidate {
    fn default() -> Self {
        Self {
            target: Entity::PLACEHOLDER,
            source: FocusSource::Proximity,
            distance: 0.0,
            alignment: 0.0,
            slot_priority: 0.0,
            target_priority: 0.0,
            score: 0.0,
        }
    }
}

#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
pub struct InteractionCandidates {
    pub entries: Vec<InteractionCandidate>,
}

#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
pub struct FocusedInteraction {
    pub target: Option<Entity>,
    pub slot_id: Option<InteractionSlotId>,
    pub source: Option<FocusSource>,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct InteractionOffer {
    pub target: Entity,
    pub slot_id: InteractionSlotId,
    pub stage_id: Option<InteractionStageId>,
    pub prompt: InteractionPrompt,
    pub availability: Option<InteractionAvailabilityReason>,
    pub suppressed: bool,
    pub source: FocusSource,
}

#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
pub struct InteractionPromptState {
    pub offer: Option<InteractionOffer>,
}

#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
pub struct InteractionFocusedBy {
    pub interactors: Vec<Entity>,
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
pub struct ActiveInteraction {
    pub target: Entity,
    pub slot_id: InteractionSlotId,
    pub stage_id: Option<InteractionStageId>,
    pub execution: InteractionExecution,
    pub progress: f32,
    pub started_at_seconds: f64,
    pub toggle_state: Option<bool>,
    pub stage_index: usize,
}

#[derive(Clone, Debug, PartialEq, Reflect)]
pub enum InteractionOccluderShape {
    Sphere { radius: f32 },
    Cuboid { half_extents: Vec3 },
    Circle2d { radius: f32 },
    Rect2d { half_extents: Vec2 },
}

#[derive(Component, Clone, Debug, PartialEq, Reflect)]
pub struct InteractionOccluder {
    pub shape: InteractionOccluderShape,
}
