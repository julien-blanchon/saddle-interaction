#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use bevy::prelude::*;
#[cfg(feature = "dev")]
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_enhanced_input::prelude::{Cancel as InputCancel, *};
use saddle_saddle_interaction::{
    ActiveInteraction, FocusedInteraction, Interactable, InteractionAvailabilityConfig,
    InteractionAvailabilityReason, InteractionCanceled, InteractionCompleted, InteractionConfig,
    InteractionDebugSettings, InteractionFailed, InteractionFocusedBy, InteractionIntent,
    InteractionIntentKind, InteractionOffered, InteractionPlugin, InteractionProgress,
    InteractionSlot, InteractionStageAdvanced, InteractionTag, InteractionTags, InteractionTarget,
    Interactor, InteractorAim,
};

const DEFAULT_BRPP_PORT: u16 = 15_732;
const INTERACTOR_SIZE: Vec2 = Vec2::new(44.0, 44.0);
const TARGET_SIZE: Vec2 = Vec2::new(92.0, 92.0);
const PRIORITY_STATION_RANGE: f32 = 160.0;
const HOLD_STATION_RANGE: f32 = 114.0;
const SYSTEM_STATION_RANGE: f32 = 126.0;

const PRIORITY_STATION_POSITION: Vec3 = Vec3::new(-210.0, 88.0, 4.0);
const HOLD_STATION_POSITION: Vec3 = Vec3::new(-18.0, 0.0, 4.0);
const MULTI_STATION_POSITION: Vec3 = Vec3::new(188.0, 120.0, 4.0);
const GATED_STATION_POSITION: Vec3 = Vec3::new(190.0, -120.0, 4.0);

const NEARBY_CRATE_POSITION: Vec3 = Vec3::new(-132.0, 60.0, 2.0);
const PRIORITY_RELAY_POSITION: Vec3 = Vec3::new(-60.0, 112.0, 2.0);
const HOLD_CONSOLE_POSITION: Vec3 = Vec3::new(90.0, 0.0, 2.0);
const MULTI_PANEL_POSITION: Vec3 = Vec3::new(312.0, 120.0, 2.0);
const GATED_DOOR_POSITION: Vec3 = Vec3::new(314.0, -120.0, 2.0);

const PRIORITY_RELAY_NAME: &str = "Priority Relay";
const HOLD_CONSOLE_NAME: &str = "Stabilizer Console";
const MULTI_PANEL_NAME: &str = "Service Panel";
const GATED_DOOR_NAME: &str = "Sealed Door";

#[derive(Component)]
struct LabInteractor;

#[derive(Component)]
struct LabTargetVisual;

#[derive(Component)]
struct LabOverlay;

#[derive(Component)]
struct LabInputContext;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LabTargetKind {
    NearbyCrate,
    PriorityRelay,
    HoldConsole,
    MultiPanel,
    GatedDoor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LabStation {
    Priority,
    Hold,
    Multi,
    Gated,
}

#[derive(Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource)]
pub struct LabDiagnostics {
    pub focused_target_name: Option<String>,
    pub focused_slot: Option<String>,
    pub prompt_label: Option<String>,
    pub availability: Option<String>,
    pub active_slot: Option<String>,
    pub active_progress: f32,
    pub completed_count: usize,
    pub canceled_count: usize,
    pub failed_count: usize,
    pub stage_advanced_count: usize,
    pub last_completed_slot: Option<String>,
    pub last_canceled_reason: Option<String>,
    pub last_failed_reason: Option<String>,
    pub last_prompt_summary: Option<String>,
    pub last_stage_transition: Option<String>,
    pub actor_powered: bool,
    pub hold_to_toggle: bool,
}

#[derive(InputAction)]
#[action_output(bool)]
struct InteractAction;

#[derive(InputAction)]
#[action_output(bool)]
struct CancelAction;

#[derive(InputAction)]
#[action_output(bool)]
struct NextSlotAction;

#[derive(InputAction)]
#[action_output(bool)]
struct PrevSlotAction;

#[derive(InputAction)]
#[action_output(bool)]
struct PriorityStationAction;

#[derive(InputAction)]
#[action_output(bool)]
struct HoldStationAction;

#[derive(InputAction)]
#[action_output(bool)]
struct MultiStationAction;

#[derive(InputAction)]
#[action_output(bool)]
struct GatedStationAction;

#[derive(InputAction)]
#[action_output(bool)]
struct TogglePoweredAction;

#[derive(InputAction)]
#[action_output(bool)]
struct ToggleAccessibilityAction;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.035, 0.04, 0.055)));
    app.insert_resource(lab_config());
    app.insert_resource(InteractionDebugSettings {
        enabled: true,
        ..default()
    });
    app.init_resource::<LabDiagnostics>();
    app.register_type::<LabDiagnostics>();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "interaction crate-local lab".into(),
            resolution: (1460, 860).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<LabInputContext>();
    #[cfg(feature = "dev")]
    app.add_plugins(BrpExtrasPlugin::with_port(lab_brp_port()));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::InteractionLabE2EPlugin);
    app.add_plugins(InteractionPlugin::default());
    app.add_observer(on_interact_start);
    app.add_observer(on_interact_release);
    app.add_observer(on_interact_cancel);
    app.add_observer(on_explicit_cancel);
    app.add_observer(on_next_slot);
    app.add_observer(on_prev_slot);
    app.add_observer(on_priority_station);
    app.add_observer(on_hold_station);
    app.add_observer(on_multi_station);
    app.add_observer(on_gated_station);
    app.add_observer(on_toggle_powered);
    app.add_observer(on_toggle_accessibility);
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            tint_targets.after(saddle_interaction::InteractionSystems::Feedback),
            record_prompt_messages.after(saddle_interaction::InteractionSystems::Feedback),
            record_completed_messages.after(saddle_interaction::InteractionSystems::Feedback),
            record_canceled_messages.after(saddle_interaction::InteractionSystems::Feedback),
            record_failed_messages.after(saddle_interaction::InteractionSystems::Feedback),
            record_stage_messages.after(saddle_interaction::InteractionSystems::Feedback),
            record_progress_messages.after(saddle_interaction::InteractionSystems::Feedback),
            update_diagnostics.after(saddle_interaction::InteractionSystems::Feedback),
            update_overlay.after(saddle_interaction::InteractionSystems::Feedback),
        ),
    );
    app.run();
}

fn lab_config() -> InteractionConfig {
    InteractionConfig {
        default_max_distance: 160.0,
        default_proximity_radius: 160.0,
        hysteresis: 0.22,
        default_input_buffer_seconds: 0.16,
        ..default()
    }
}

#[cfg(feature = "dev")]
fn lab_brp_port() -> u16 {
    std::env::var("BRP_EXTRAS_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_BRPP_PORT)
}

fn setup(mut commands: Commands) {
    commands.spawn((Name::new("Lab Camera"), Camera2d));

    spawn_zone(
        &mut commands,
        "Priority Zone",
        Vec3::new(-120.0, 88.0, -1.0),
        Vec2::new(330.0, 240.0),
        Color::srgb(0.11, 0.13, 0.16),
    );
    spawn_zone(
        &mut commands,
        "Hold Zone",
        Vec3::new(35.0, 0.0, -1.0),
        Vec2::new(250.0, 210.0),
        Color::srgb(0.10, 0.12, 0.15),
    );
    spawn_zone(
        &mut commands,
        "Systems Zone",
        Vec3::new(295.0, 0.0, -1.0),
        Vec2::new(300.0, 410.0),
        Color::srgb(0.10, 0.11, 0.14),
    );

    commands.spawn((
        Name::new("Interactor"),
        LabInteractor,
        LabInputContext,
        Interactor {
            max_distance: Some(PRIORITY_STATION_RANGE),
            proximity_radius: Some(PRIORITY_STATION_RANGE),
            hysteresis: Some(0.24),
            distance_weight: 1.0,
            alignment_weight: 0.2,
            target_priority_weight: 1.0,
            slot_priority_weight: 0.5,
            ..default()
        },
        InteractorAim {
            direction: station_profile(LabStation::Priority).1,
        },
        InteractionTags::default(),
        Sprite {
            color: Color::srgb(0.14, 0.80, 0.96),
            custom_size: Some(INTERACTOR_SIZE),
            ..default()
        },
        Transform::from_translation(PRIORITY_STATION_POSITION),
        GlobalTransform::from_translation(PRIORITY_STATION_POSITION),
        actions!(LabInputContext[
            (
                Action::<InteractAction>::new(),
                bindings![KeyCode::KeyE, GamepadButton::South],
            ),
            (
                Action::<CancelAction>::new(),
                bindings![KeyCode::Escape, GamepadButton::East],
            ),
            (
                Action::<NextSlotAction>::new(),
                bindings![KeyCode::Tab],
            ),
            (
                Action::<PrevSlotAction>::new(),
                bindings![KeyCode::KeyQ],
            ),
            (
                Action::<PriorityStationAction>::new(),
                bindings![KeyCode::Digit1],
            ),
            (
                Action::<HoldStationAction>::new(),
                bindings![KeyCode::Digit2],
            ),
            (
                Action::<MultiStationAction>::new(),
                bindings![KeyCode::Digit3],
            ),
            (
                Action::<GatedStationAction>::new(),
                bindings![KeyCode::Digit4],
            ),
            (
                Action::<TogglePoweredAction>::new(),
                bindings![KeyCode::KeyP],
            ),
            (
                Action::<ToggleAccessibilityAction>::new(),
                bindings![KeyCode::KeyT],
            ),
        ]),
    ));

    spawn_target(
        &mut commands,
        "Nearby Crate",
        LabTargetKind::NearbyCrate,
        NEARBY_CRATE_POSITION,
        Color::srgb(0.44, 0.49, 0.56),
        Interactable::default(),
        InteractionTarget {
            slots: vec![InteractionSlot::instant("inspect_crate", "Inspect")],
        },
    );
    spawn_target(
        &mut commands,
        PRIORITY_RELAY_NAME,
        LabTargetKind::PriorityRelay,
        PRIORITY_RELAY_POSITION,
        Color::srgb(0.24, 0.55, 0.96),
        Interactable {
            priority: 0.8,
            ..default()
        },
        InteractionTarget {
            slots: vec![InteractionSlot {
                priority: 0.7,
                ..InteractionSlot::instant("reroute", "Reroute")
            }],
        },
    );
    spawn_target(
        &mut commands,
        HOLD_CONSOLE_NAME,
        LabTargetKind::HoldConsole,
        HOLD_CONSOLE_POSITION,
        Color::srgb(0.86, 0.54, 0.24),
        Interactable::default(),
        InteractionTarget {
            slots: vec![InteractionSlot {
                behavior: saddle_interaction::InteractionBehavior::Single(
                    saddle_interaction::InteractionExecution::Hold {
                        duration_seconds: 0.75,
                    },
                ),
                ..InteractionSlot::instant("stabilize", "Stabilize")
            }],
        },
    );
    spawn_target(
        &mut commands,
        MULTI_PANEL_NAME,
        LabTargetKind::MultiPanel,
        MULTI_PANEL_POSITION,
        Color::srgb(0.40, 0.68, 0.42),
        Interactable::default(),
        InteractionTarget {
            slots: vec![
                InteractionSlot {
                    priority: 1.1,
                    ..InteractionSlot::instant("hack", "Hack")
                },
                InteractionSlot {
                    priority: 0.1,
                    ..InteractionSlot::instant("read", "Read")
                },
            ],
        },
    );
    spawn_target(
        &mut commands,
        GATED_DOOR_NAME,
        LabTargetKind::GatedDoor,
        GATED_DOOR_POSITION,
        Color::srgb(0.70, 0.30, 0.32),
        Interactable {
            priority: 0.3,
            ..default()
        },
        InteractionTarget {
            slots: vec![InteractionSlot {
                availability: InteractionAvailabilityConfig {
                    required_actor_tags: vec![InteractionTag::from("powered")],
                    ..default()
                },
                ..InteractionSlot::instant("unlock_door", "Unlock Door")
            }],
        },
    );

    commands.spawn((
        Name::new("Lab Overlay"),
        LabOverlay,
        Text::new(String::new()),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(16.0),
            width: px(450.0),
            ..default()
        },
        TextFont {
            font_size: 17.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn spawn_zone(commands: &mut Commands, name: &str, position: Vec3, size: Vec2, color: Color) {
    commands.spawn((
        Name::new(name.to_owned()),
        Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(position),
        GlobalTransform::from_translation(position),
    ));
}

fn spawn_target(
    commands: &mut Commands,
    name: &str,
    kind: LabTargetKind,
    position: Vec3,
    color: Color,
    interactable: Interactable,
    target: InteractionTarget,
) {
    commands.spawn((
        Name::new(name.to_owned()),
        kind,
        LabTargetVisual,
        interactable,
        target,
        Sprite {
            color,
            custom_size: Some(TARGET_SIZE),
            ..default()
        },
        Transform::from_translation(position),
        GlobalTransform::from_translation(position),
    ));
}

fn on_interact_start(
    trigger: On<Start<InteractAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::Press,
    });
}

fn on_interact_release(
    trigger: On<Complete<InteractAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::Release,
    });
}

fn on_interact_cancel(
    trigger: On<InputCancel<InteractAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::Cancel,
    });
}

fn on_explicit_cancel(
    trigger: On<Start<CancelAction>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::Cancel,
    });
}

fn on_next_slot(trigger: On<Start<NextSlotAction>>, mut intents: MessageWriter<InteractionIntent>) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::CycleNext,
    });
}

fn on_prev_slot(trigger: On<Start<PrevSlotAction>>, mut intents: MessageWriter<InteractionIntent>) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::CyclePrevious,
    });
}

fn on_priority_station(
    trigger: On<Start<PriorityStationAction>>,
    mut commands: Commands,
    interactors: Query<&Interactor, With<LabInteractor>>,
) {
    queue_station_change(
        &mut commands,
        &interactors,
        trigger.context,
        LabStation::Priority,
    );
}

fn on_hold_station(
    trigger: On<Start<HoldStationAction>>,
    mut commands: Commands,
    interactors: Query<&Interactor, With<LabInteractor>>,
) {
    queue_station_change(
        &mut commands,
        &interactors,
        trigger.context,
        LabStation::Hold,
    );
}

fn on_multi_station(
    trigger: On<Start<MultiStationAction>>,
    mut commands: Commands,
    interactors: Query<&Interactor, With<LabInteractor>>,
) {
    queue_station_change(
        &mut commands,
        &interactors,
        trigger.context,
        LabStation::Multi,
    );
}

fn on_gated_station(
    trigger: On<Start<GatedStationAction>>,
    mut commands: Commands,
    interactors: Query<&Interactor, With<LabInteractor>>,
) {
    queue_station_change(
        &mut commands,
        &interactors,
        trigger.context,
        LabStation::Gated,
    );
}

fn queue_station_change(
    commands: &mut Commands,
    interactors: &Query<&Interactor, With<LabInteractor>>,
    interactor_entity: Entity,
    station: LabStation,
) {
    let Ok(interactor_config) = interactors.get(interactor_entity) else {
        return;
    };
    let interactor_config = interactor_config.clone();
    let (position, aim, range) = station_profile(station);
    commands.entity(interactor_entity).insert((
        Transform::from_translation(position),
        GlobalTransform::from_translation(position),
        InteractorAim { direction: aim },
        Interactor {
            max_distance: Some(range),
            proximity_radius: Some(range),
            ..interactor_config
        },
    ));
}

fn on_toggle_powered(
    trigger: On<Start<TogglePoweredAction>>,
    mut commands: Commands,
    tags_query: Query<&InteractionTags, With<LabInteractor>>,
) {
    let Ok(existing) = tags_query.get(trigger.context) else {
        return;
    };
    let existing = existing.clone();
    let powered = InteractionTag::from("powered");
    let mut next = existing;
    if next.contains(&powered) {
        next.tags.retain(|tag| tag != &powered);
    } else {
        next.tags.push(powered);
    }
    commands.entity(trigger.context).insert(next);
}

fn on_toggle_accessibility(
    _trigger: On<Start<ToggleAccessibilityAction>>,
    mut config: ResMut<InteractionConfig>,
) {
    config.hold_to_toggle = !config.hold_to_toggle;
}

fn tint_targets(
    active: Query<&ActiveInteraction, With<LabInteractor>>,
    mut targets: Query<
        (
            Entity,
            &LabTargetKind,
            &mut Sprite,
            Option<&InteractionFocusedBy>,
        ),
        With<LabTargetVisual>,
    >,
) {
    let active_state = active
        .iter()
        .next()
        .map(|entry| (entry.target, entry.progress));

    for (entity, kind, mut sprite, focused_by) in &mut targets {
        let mut color = base_target_color(*kind);
        if let Some((active_target, progress)) = active_state
            && active_target == entity
        {
            color = if progress >= 1.0 {
                Color::srgb(0.22, 0.86, 0.54)
            } else if progress >= 0.5 {
                Color::srgb(0.95, 0.72, 0.26)
            } else {
                Color::srgb(0.90, 0.56, 0.24)
            };
        }
        if focused_by.is_some_and(|focus| !focus.interactors.is_empty()) {
            color = Color::srgb(0.98, 0.82, 0.24);
        }
        sprite.color = color;
    }
}

fn base_target_color(kind: LabTargetKind) -> Color {
    match kind {
        LabTargetKind::NearbyCrate => Color::srgb(0.44, 0.49, 0.56),
        LabTargetKind::PriorityRelay => Color::srgb(0.24, 0.55, 0.96),
        LabTargetKind::HoldConsole => Color::srgb(0.86, 0.54, 0.24),
        LabTargetKind::MultiPanel => Color::srgb(0.40, 0.68, 0.42),
        LabTargetKind::GatedDoor => Color::srgb(0.70, 0.30, 0.32),
    }
}

fn record_prompt_messages(
    mut diagnostics: ResMut<LabDiagnostics>,
    mut reader: MessageReader<InteractionOffered>,
) {
    for event in reader.read() {
        diagnostics.last_prompt_summary = event.offer.as_ref().map(|offer| {
            let availability = offer
                .availability
                .as_ref()
                .map(format_availability)
                .unwrap_or_else(|| "available".to_owned());
            format!("{} [{availability}]", offer.prompt.action_label_key)
        });
    }
}

fn record_completed_messages(
    mut diagnostics: ResMut<LabDiagnostics>,
    mut reader: MessageReader<InteractionCompleted>,
) {
    for event in reader.read() {
        diagnostics.completed_count += 1;
        diagnostics.last_completed_slot = Some(event.slot_id.0.clone());
    }
}

fn record_canceled_messages(
    mut diagnostics: ResMut<LabDiagnostics>,
    mut reader: MessageReader<InteractionCanceled>,
) {
    for event in reader.read() {
        diagnostics.canceled_count += 1;
        diagnostics.last_canceled_reason = Some(format_cancel_reason(&event.reason));
    }
}

fn record_failed_messages(
    mut diagnostics: ResMut<LabDiagnostics>,
    mut reader: MessageReader<InteractionFailed>,
) {
    for event in reader.read() {
        diagnostics.failed_count += 1;
        diagnostics.last_failed_reason = Some(format_availability(&event.reason));
    }
}

fn record_stage_messages(
    mut diagnostics: ResMut<LabDiagnostics>,
    mut reader: MessageReader<InteractionStageAdvanced>,
) {
    for event in reader.read() {
        diagnostics.stage_advanced_count += 1;
        diagnostics.last_stage_transition = Some(format!(
            "{} -> {}",
            event
                .previous_stage_id
                .as_ref()
                .map(|id| id.0.as_str())
                .unwrap_or("start"),
            event
                .next_stage_id
                .as_ref()
                .map(|id| id.0.as_str())
                .unwrap_or("terminal")
        ));
    }
}

fn record_progress_messages(
    mut diagnostics: ResMut<LabDiagnostics>,
    mut reader: MessageReader<InteractionProgress>,
) {
    for event in reader.read() {
        diagnostics.active_slot = Some(event.slot_id.0.clone());
        diagnostics.active_progress = event.progress;
    }
}

fn update_diagnostics(
    config: Res<InteractionConfig>,
    names: Query<&Name>,
    interactor_tags: Query<&InteractionTags, With<LabInteractor>>,
    focused: Query<&FocusedInteraction, With<LabInteractor>>,
    prompt: Query<&saddle_interaction::InteractionPromptState, With<LabInteractor>>,
    active: Query<&ActiveInteraction, With<LabInteractor>>,
    mut diagnostics: ResMut<LabDiagnostics>,
) {
    let focused = focused.iter().next().cloned().unwrap_or_default();
    let prompt = prompt
        .iter()
        .next()
        .and_then(|state| state.offer.as_ref().cloned());
    let active = active.iter().next().cloned();

    diagnostics.focused_target_name = focused
        .target
        .and_then(|entity| names.get(entity).ok().map(|name| name.as_str().to_owned()));
    diagnostics.focused_slot = focused.slot_id.as_ref().map(|slot| slot.0.clone());
    diagnostics.prompt_label = prompt
        .as_ref()
        .map(|offer| offer.prompt.action_label_key.clone());
    diagnostics.availability = prompt
        .as_ref()
        .and_then(|offer| offer.availability.as_ref().map(format_availability));
    diagnostics.active_slot = active.as_ref().map(|entry| entry.slot_id.0.clone());
    diagnostics.active_progress = active.as_ref().map_or(0.0, |entry| entry.progress);
    diagnostics.actor_powered = interactor_tags
        .iter()
        .next()
        .is_some_and(|tags| tags.contains(&InteractionTag::from("powered")));
    diagnostics.hold_to_toggle = config.hold_to_toggle;
}

fn update_overlay(
    diagnostics: Res<LabDiagnostics>,
    mut overlay: Single<&mut Text, With<LabOverlay>>,
) {
    overlay.0 = format!(
        "interaction lab\n\
         stations: arbitration | hold | multi-slot | gated\n\
         controls: 1-4 jump stations | E confirm | Esc cancel | Tab/Q cycle | P power | T accessibility\n\
         focus: {}\n\
         slot: {}\n\
         prompt: {}\n\
         availability: {}\n\
         active: {} ({:.0}%)\n\
         last offered: {}\n\
         last completed: {}\n\
         last canceled: {}\n\
         last failed: {}\n\
         stage: {}\n\
         powered tag: {}\n\
         hold_to_toggle: {}\n\
         completed: {} canceled: {} failed: {} stage advances: {}",
        diagnostics.focused_target_name.as_deref().unwrap_or("none"),
        diagnostics.focused_slot.as_deref().unwrap_or("none"),
        diagnostics.prompt_label.as_deref().unwrap_or("none"),
        diagnostics.availability.as_deref().unwrap_or("available"),
        diagnostics.active_slot.as_deref().unwrap_or("none"),
        diagnostics.active_progress * 100.0,
        diagnostics.last_prompt_summary.as_deref().unwrap_or("none"),
        diagnostics.last_completed_slot.as_deref().unwrap_or("none"),
        diagnostics
            .last_canceled_reason
            .as_deref()
            .unwrap_or("none"),
        diagnostics.last_failed_reason.as_deref().unwrap_or("none"),
        diagnostics
            .last_stage_transition
            .as_deref()
            .unwrap_or("none"),
        diagnostics.actor_powered,
        diagnostics.hold_to_toggle,
        diagnostics.completed_count,
        diagnostics.canceled_count,
        diagnostics.failed_count,
        diagnostics.stage_advanced_count,
    );
}

fn format_availability(reason: &InteractionAvailabilityReason) -> String {
    match reason {
        InteractionAvailabilityReason::Disabled => "disabled".to_owned(),
        InteractionAvailabilityReason::Busy => "busy".to_owned(),
        InteractionAvailabilityReason::ReservedByOther => "reserved".to_owned(),
        InteractionAvailabilityReason::Consumed => "consumed".to_owned(),
        InteractionAvailabilityReason::MissingActorTag(tag) => {
            format!("missing_actor_tag:{}", tag.0)
        }
        InteractionAvailabilityReason::BlockedActorTag(tag) => {
            format!("blocked_actor_tag:{}", tag.0)
        }
        InteractionAvailabilityReason::MissingTargetTag(tag) => {
            format!("missing_target_tag:{}", tag.0)
        }
        InteractionAvailabilityReason::BlockedTargetTag(tag) => {
            format!("blocked_target_tag:{}", tag.0)
        }
        InteractionAvailabilityReason::SharedCooldown { .. } => "shared_cooldown".to_owned(),
        InteractionAvailabilityReason::PerActorCooldown { .. } => "per_actor_cooldown".to_owned(),
        InteractionAvailabilityReason::PredicateFailed { predicate, .. } => {
            format!("predicate_failed:{}", predicate.0)
        }
        InteractionAvailabilityReason::OutOfRange => "out_of_range".to_owned(),
        InteractionAvailabilityReason::LineOfSightBlocked => "line_of_sight_blocked".to_owned(),
        InteractionAvailabilityReason::MissingTarget => "missing_target".to_owned(),
        InteractionAvailabilityReason::NoSlots => "no_slots".to_owned(),
    }
}

fn format_cancel_reason(reason: &saddle_interaction::InteractionCancelReason) -> String {
    match reason {
        saddle_interaction::InteractionCancelReason::ExplicitCancel => "explicit_cancel".to_owned(),
        saddle_interaction::InteractionCancelReason::InputReleased => "input_released".to_owned(),
        saddle_interaction::InteractionCancelReason::FocusLost => "focus_lost".to_owned(),
        saddle_interaction::InteractionCancelReason::DistanceBreak => "distance_break".to_owned(),
        saddle_interaction::InteractionCancelReason::LineOfSightBreak => "line_of_sight_break".to_owned(),
        saddle_interaction::InteractionCancelReason::Busy => "busy".to_owned(),
        saddle_interaction::InteractionCancelReason::TargetMissing => "target_missing".to_owned(),
        saddle_interaction::InteractionCancelReason::ReservationLost => "reservation_lost".to_owned(),
        saddle_interaction::InteractionCancelReason::PredicateInvalidated { predicate, .. } => {
            format!("predicate_invalidated:{}", predicate.0)
        }
        saddle_interaction::InteractionCancelReason::Other(detail) => format!("other:{detail}"),
    }
}

fn station_profile(station: LabStation) -> (Vec3, Vec3, f32) {
    match station {
        LabStation::Priority => (
            PRIORITY_STATION_POSITION,
            (PRIORITY_RELAY_POSITION - PRIORITY_STATION_POSITION).normalize_or_zero(),
            PRIORITY_STATION_RANGE,
        ),
        LabStation::Hold => (
            HOLD_STATION_POSITION,
            (HOLD_CONSOLE_POSITION - HOLD_STATION_POSITION).normalize_or_zero(),
            HOLD_STATION_RANGE,
        ),
        LabStation::Multi => (
            MULTI_STATION_POSITION,
            (MULTI_PANEL_POSITION - MULTI_STATION_POSITION).normalize_or_zero(),
            SYSTEM_STATION_RANGE,
        ),
        LabStation::Gated => (
            GATED_STATION_POSITION,
            (GATED_DOOR_POSITION - GATED_STATION_POSITION).normalize_or_zero(),
            SYSTEM_STATION_RANGE,
        ),
    }
}

fn interactor_entity(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<LabInteractor>>()
        .single(world)
        .expect("lab interactor should exist")
}

pub fn go_to_station(world: &mut World, station: LabStation) {
    let interactor = interactor_entity(world);
    let (position, aim, range) = station_profile(station);
    let interactor_config = world
        .get::<Interactor>(interactor)
        .cloned()
        .expect("lab interactor should have an Interactor component");
    world.entity_mut(interactor).insert((
        Transform::from_translation(position),
        GlobalTransform::from_translation(position),
        InteractorAim { direction: aim },
        Interactor {
            max_distance: Some(range),
            proximity_radius: Some(range),
            ..interactor_config
        },
    ));
}

pub fn send_intent(world: &mut World, kind: InteractionIntentKind) {
    let interactor = interactor_entity(world);
    world.write_message(InteractionIntent { interactor, kind });
}

pub fn set_accessibility_toggle(world: &mut World, enabled: bool) {
    world.resource_mut::<InteractionConfig>().hold_to_toggle = enabled;
}

pub fn set_actor_powered(world: &mut World, enabled: bool) {
    let interactor = interactor_entity(world);
    let mut tags = world
        .get::<InteractionTags>(interactor)
        .cloned()
        .unwrap_or_default();
    let powered = InteractionTag::from("powered");

    tags.tags.retain(|tag| tag != &powered);
    if enabled {
        tags.tags.push(powered);
    }

    world.entity_mut(interactor).insert(tags);
}
