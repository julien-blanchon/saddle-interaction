use bevy::prelude::*;
use bevy_enhanced_input::prelude::{Cancel as InputCancel, *};
use saddle_interaction::{
    ActiveInteraction, Interactable, InteractionBehavior, InteractionCompleted, InteractionConfig,
    InteractionExecution, InteractionFailed, InteractionFocusedBy, InteractionIntent,
    InteractionIntentKind, InteractionOffered, InteractionPlugin, InteractionProgress,
    InteractionPromptState, InteractionReservationPolicy, InteractionSlot, InteractionStage,
    InteractionStageAdvanced, InteractionTag, InteractionTags, InteractionTarget, Interactor,
};
use saddle_pane::prelude::*;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemoMode {
    Basic,
    Hold,
    MultiAction,
    Chained,
    Gated,
    Accessibility,
    PromptUi,
    VehicleBay,
}

impl DemoMode {
    fn title(self) -> &'static str {
        match self {
            DemoMode::Basic => "interaction/basic",
            DemoMode::Hold => "interaction/hold",
            DemoMode::MultiAction => "interaction/multi_action",
            DemoMode::Chained => "interaction/chained",
            DemoMode::Gated => "interaction/gated",
            DemoMode::Accessibility => "interaction/accessibility",
            DemoMode::PromptUi => "interaction/prompt_ui",
            DemoMode::VehicleBay => "interaction/vehicle_bay",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            DemoMode::Basic => "Press E to complete a single instant interaction.",
            DemoMode::Hold => "Hold E until the progress reaches 100%.",
            DemoMode::MultiAction => {
                "Use Tab / Q to cycle slots on the same target, then press E to trigger the selected slot."
            }
            DemoMode::Chained => "Press E repeatedly to advance through the stage sequence.",
            DemoMode::Gated => "Power the actor first, then use the gated slot.",
            DemoMode::Accessibility => "Hold interactions are converted into toggles by config.",
            DemoMode::PromptUi => {
                "The HUD is driven only from InteractionPromptState and lifecycle messages."
            }
            DemoMode::VehicleBay => {
                "Enter the rover, occupy the exclusive seat, then use the exit hatch to step back out."
            }
        }
    }

    fn config(self) -> InteractionConfig {
        match self {
            DemoMode::Accessibility => InteractionConfig {
                hold_to_toggle: true,
                mash_auto_complete: true,
                ..default()
            },
            _ => InteractionConfig::default(),
        }
    }
}

#[derive(Component)]
struct DemoInteractorContext;

#[derive(Component)]
pub struct DemoInteractor;

#[derive(Component)]
struct DemoTargetVisual;

#[derive(Component)]
struct DemoOverlay;

#[derive(Component, Clone)]
pub struct DemoBaseTargetSlots(pub Vec<InteractionSlot>);

#[derive(Resource, Default)]
struct InteractionDemoPaneInstalled;

#[derive(Resource, Default)]
struct DemoLog {
    last_prompt: String,
    last_result: String,
    progress: f32,
    completed: usize,
    failed: usize,
    canceled: usize,
    stage_advanced: usize,
}

#[derive(Resource, Clone, Copy)]
struct DemoModeResource(DemoMode);

#[derive(Resource, Clone, Default, Pane)]
#[pane(title = "Interaction Tuning")]
struct InteractionDemoPane {
    #[pane(slider, min = 2.0, max = 12.0, step = 0.1)]
    actor_range: f32,
    #[pane(slider, min = 0.5, max = 2.5, step = 0.05)]
    detection_radius_scale: f32,
    #[pane(slider, min = 0.25, max = 2.5, step = 0.05)]
    hold_time_scale: f32,
    hold_to_toggle: bool,
    auto_interact_on_focus: bool,
}

impl InteractionDemoPane {
    fn for_mode(mode: DemoMode) -> Self {
        Self {
            actor_range: 6.0,
            detection_radius_scale: 1.0,
            hold_time_scale: 1.0,
            hold_to_toggle: mode == DemoMode::Accessibility,
            auto_interact_on_focus: false,
        }
    }
}

#[derive(InputAction)]
#[action_output(bool)]
struct InteractAction;

#[derive(InputAction)]
#[action_output(bool)]
struct CancelAction;

#[derive(InputAction)]
#[action_output(bool)]
struct NextAction;

#[derive(InputAction)]
#[action_output(bool)]
struct PrevAction;

pub fn run(mode: DemoMode) {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)));
    app.insert_resource(DemoLog::default());
    app.insert_resource(DemoModeResource(mode));
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: mode.title().into(),
            resolution: (1100, 720).into(),
            ..default()
        }),
        ..default()
    }));
    install_demo_pane(&mut app, mode == DemoMode::Accessibility);
    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<DemoInteractorContext>();
    app.add_plugins(InteractionPlugin::default().with_config(mode.config()));
    app.add_observer(on_interact_start);
    app.add_observer(on_interact_release);
    app.add_observer(on_interact_cancel);
    app.add_observer(on_explicit_cancel);
    app.add_observer(on_next_slot);
    app.add_observer(on_prev_slot);
    app.add_systems(Startup, setup_scene);
    app.add_systems(
        Update,
        (
            tint_targets,
            update_overlay,
            record_prompt_messages,
            record_completed_messages,
            record_failed_messages,
            record_canceled_messages,
            record_progress_messages,
            record_stage_messages,
            gate_example_unlocks,
        ),
    );
    app.run();
}

fn setup_scene(mut commands: Commands, mode: Res<DemoModeResource>) {
    commands.spawn((Name::new("Demo Camera"), Camera2d));
    commands.spawn((
        Name::new("Backdrop"),
        Sprite::from_color(Color::srgb(0.07, 0.08, 0.11), Vec2::new(2200.0, 1600.0)),
        Transform::from_xyz(0.0, 0.0, -20.0),
    ));
    commands.spawn((
        Name::new("Upper Band"),
        Sprite::from_color(Color::srgb(0.08, 0.1, 0.14), Vec2::new(2200.0, 260.0)),
        Transform::from_xyz(0.0, 240.0, -18.0),
    ));
    commands.spawn((
        Name::new("Lower Pad"),
        Sprite::from_color(Color::srgb(0.1, 0.11, 0.14), Vec2::new(2200.0, 220.0)),
        Transform::from_xyz(0.0, -250.0, -18.0),
    ));
    for (index, x) in (-5..=5).enumerate() {
        commands.spawn((
            Name::new(format!("Guide Column {}", index + 1)),
            Sprite::from_color(Color::srgba(0.85, 0.92, 1.0, 0.035), Vec2::new(2.0, 1600.0)),
            Transform::from_xyz(x as f32 * 110.0, 0.0, -15.0),
        ));
    }
    commands.spawn((
        Name::new("Demo Overlay"),
        DemoOverlay,
        Text::new(String::new()),
        Node {
            position_type: PositionType::Absolute,
            left: px(20.0),
            top: px(18.0),
            width: px(540.0),
            ..default()
        },
        TextFont {
            font_size: 17.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));

    commands.spawn((
        Name::new("Interactor"),
        DemoInteractor,
        DemoInteractorContext,
        Interactor {
            max_distance: Some(6.0),
            proximity_radius: Some(6.0),
            ..default()
        },
        InteractionTags::default(),
        Sprite {
            color: Color::srgb(0.16, 0.78, 0.96),
            custom_size: Some(Vec2::new(54.0, 54.0)),
            ..default()
        },
        Transform::from_xyz(-260.0, 0.0, 2.0),
        GlobalTransform::from_xyz(-260.0, 0.0, 2.0),
        actions!(DemoInteractorContext[
            (
                Action::<InteractAction>::new(),
                bindings![KeyCode::KeyE, GamepadButton::South],
            ),
            (
                Action::<CancelAction>::new(),
                bindings![KeyCode::Escape, GamepadButton::East],
            ),
            (
                Action::<NextAction>::new(),
                bindings![KeyCode::Tab],
            ),
            (
                Action::<PrevAction>::new(),
                bindings![KeyCode::KeyQ],
            ),
        ]),
    ));

    match mode.0 {
        DemoMode::Basic | DemoMode::PromptUi => {
            spawn_target(
                &mut commands,
                "Terminal",
                Vec3::new(110.0, 0.0, 1.0),
                vec![InteractionSlot::instant("inspect", "Inspect")],
            );
        }
        DemoMode::Hold => {
            spawn_target(
                &mut commands,
                "Bulkhead",
                Vec3::new(110.0, 0.0, 1.0),
                vec![InteractionSlot {
                    behavior: InteractionBehavior::Single(
                        saddle_interaction::InteractionExecution::Hold {
                            duration_seconds: 1.2,
                        },
                    ),
                    ..InteractionSlot::instant("breach", "Hold Breach")
                }],
            );
        }
        DemoMode::MultiAction => {
            spawn_target(
                &mut commands,
                "Console",
                Vec3::new(110.0, 0.0, 1.0),
                vec![
                    InteractionSlot {
                        priority: 0.1,
                        ..InteractionSlot::instant("read", "Read")
                    },
                    InteractionSlot {
                        priority: 1.0,
                        ..InteractionSlot::instant("hack", "Hack")
                    },
                ],
            );
        }
        DemoMode::Chained => {
            spawn_target(
                &mut commands,
                "Lever",
                Vec3::new(110.0, 0.0, 1.0),
                vec![InteractionSlot {
                    behavior: InteractionBehavior::Sequence {
                        stages: vec![
                            InteractionStage {
                                id: "stage_a".into(),
                                execution: saddle_interaction::InteractionExecution::Instant,
                                prompt: Some(saddle_interaction::InteractionPrompt {
                                    action_label_key: "Prime".into(),
                                    ..default()
                                }),
                            },
                            InteractionStage {
                                id: "stage_b".into(),
                                execution: saddle_interaction::InteractionExecution::Instant,
                                prompt: Some(saddle_interaction::InteractionPrompt {
                                    action_label_key: "Release".into(),
                                    ..default()
                                }),
                            },
                            InteractionStage {
                                id: "stage_c".into(),
                                execution: saddle_interaction::InteractionExecution::Instant,
                                prompt: Some(saddle_interaction::InteractionPrompt {
                                    action_label_key: "Reset".into(),
                                    ..default()
                                }),
                            },
                        ],
                        advance_mode: saddle_interaction::SequenceAdvanceMode::Loop,
                    },
                    ..InteractionSlot::instant("lever", "Prime")
                }],
            );
        }
        DemoMode::Gated => {
            spawn_target(
                &mut commands,
                "Fuse Box",
                Vec3::new(20.0, -90.0, 1.0),
                vec![InteractionSlot::instant("power_on", "Install Fuse")],
            );
            spawn_target(
                &mut commands,
                "Security Door",
                Vec3::new(160.0, 70.0, 1.0),
                vec![InteractionSlot {
                    availability: saddle_interaction::InteractionAvailabilityConfig {
                        required_actor_tags: vec![InteractionTag::from("powered")],
                        ..default()
                    },
                    ..InteractionSlot::instant("open", "Open Door")
                }],
            );
        }
        DemoMode::Accessibility => {
            spawn_target(
                &mut commands,
                "Crank",
                Vec3::new(110.0, 0.0, 1.0),
                vec![InteractionSlot {
                    behavior: InteractionBehavior::Single(
                        saddle_interaction::InteractionExecution::Hold {
                            duration_seconds: 1.5,
                        },
                    ),
                    ..InteractionSlot::instant("stabilize", "Stabilize")
                }],
            );
        }
        DemoMode::VehicleBay => {
            commands.spawn((
                Name::new("Rover Body"),
                Sprite::from_color(Color::srgb(0.21, 0.24, 0.3), Vec2::new(240.0, 180.0)),
                Transform::from_xyz(120.0, 0.0, 0.5),
            ));
            commands.spawn((
                Name::new("Rover Canopy"),
                Sprite::from_color(Color::srgba(0.3, 0.72, 0.92, 0.18), Vec2::new(160.0, 58.0)),
                Transform::from_xyz(120.0, 62.0, 0.6),
            ));
            spawn_target(
                &mut commands,
                "Cockpit",
                Vec3::new(120.0, 70.0, 1.0),
                vec![InteractionSlot {
                    availability: saddle_interaction::InteractionAvailabilityConfig {
                        blocked_actor_tags: vec![InteractionTag::from("seated")],
                        ..default()
                    },
                    reservation: InteractionReservationPolicy::Exclusive,
                    ..InteractionSlot::instant("enter_vehicle", "Enter Rover")
                }],
            );
            spawn_target(
                &mut commands,
                "Exit Hatch",
                Vec3::new(120.0, -70.0, 1.0),
                vec![InteractionSlot {
                    availability: saddle_interaction::InteractionAvailabilityConfig {
                        required_actor_tags: vec![InteractionTag::from("seated")],
                        ..default()
                    },
                    reservation: InteractionReservationPolicy::Exclusive,
                    ..InteractionSlot::instant("exit_vehicle", "Exit Rover")
                }],
            );
        }
    }
}

fn spawn_target(
    commands: &mut Commands,
    name: &str,
    position: Vec3,
    slots: Vec<InteractionSlot>,
) -> Entity {
    commands
        .spawn((
            Name::new(name.to_owned()),
            DemoTargetVisual,
            DemoBaseTargetSlots(slots.clone()),
            Interactable::default(),
            InteractionTarget { slots },
            Sprite {
                color: Color::srgb(0.30, 0.34, 0.42),
                custom_size: Some(Vec2::new(92.0, 92.0)),
                ..default()
            },
            Transform::from_translation(position),
            GlobalTransform::from_translation(position),
        ))
        .id()
}

pub fn pane_plugins() -> (
    bevy_flair::FlairPlugin,
    bevy_input_focus::InputDispatchPlugin,
    bevy_ui_widgets::UiWidgetsPlugins,
    bevy_input_focus::tab_navigation::TabNavigationPlugin,
    saddle_pane::PanePlugin,
) {
    (
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        saddle_pane::PanePlugin,
    )
}

pub fn install_demo_pane(app: &mut App, hold_to_toggle: bool) {
    if app.world().contains_resource::<InteractionDemoPaneInstalled>() {
        return;
    }

    app.insert_resource(InteractionDemoPaneInstalled);
    if !app.world().contains_resource::<InteractionDemoPane>() {
        let mut pane = InteractionDemoPane::for_mode(DemoMode::Basic);
        pane.hold_to_toggle = hold_to_toggle;
        app.insert_resource(pane);
    }
    if !app.is_plugin_added::<saddle_pane::PanePlugin>() {
        app.add_plugins(pane_plugins());
    }
    app.register_pane::<InteractionDemoPane>();
    app.add_systems(Update, sync_interaction_pane);
}

fn sync_interaction_pane(
    pane: Res<InteractionDemoPane>,
    mut config: ResMut<InteractionConfig>,
    mut interactors: Query<&mut Interactor, With<DemoInteractor>>,
    mut targets: Query<(&mut Interactable, &mut InteractionTarget, &DemoBaseTargetSlots)>,
) {
    if !pane.is_changed() {
        return;
    }

    config.hold_to_toggle = pane.hold_to_toggle;
    config.auto_interact_on_focus = pane.auto_interact_on_focus;
    config.detection_radius_scale = pane.detection_radius_scale;
    config.hold_time_scale = pane.hold_time_scale;

    for mut interactor in &mut interactors {
        interactor.max_distance = Some(pane.actor_range);
        interactor.proximity_radius = Some(pane.actor_range);
    }

    for (mut interactable, mut target, base_slots) in &mut targets {
        interactable.focus_radius = Some(pane.actor_range);
        target.slots = base_slots.0.clone();
        for slot in &mut target.slots {
            slot.auto_trigger_on_focus = pane.auto_interact_on_focus;
            scale_hold_durations(&mut slot.behavior, pane.hold_time_scale);
        }
    }
}

fn scale_hold_durations(behavior: &mut InteractionBehavior, scale: f32) {
    match behavior {
        InteractionBehavior::Single(InteractionExecution::Hold { duration_seconds })
        | InteractionBehavior::Single(InteractionExecution::Passive { duration_seconds }) => {
            *duration_seconds *= scale;
        }
        InteractionBehavior::Single(_) => {}
        InteractionBehavior::Sequence { stages, .. } => {
            for stage in stages {
                match &mut stage.execution {
                    InteractionExecution::Hold { duration_seconds }
                    | InteractionExecution::Passive { duration_seconds } => {
                        *duration_seconds *= scale;
                    }
                    _ => {}
                }
            }
        }
    }
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

fn on_next_slot(trigger: On<Start<NextAction>>, mut intents: MessageWriter<InteractionIntent>) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::CycleNext,
    });
}

fn on_prev_slot(trigger: On<Start<PrevAction>>, mut intents: MessageWriter<InteractionIntent>) {
    intents.write(InteractionIntent {
        interactor: trigger.context,
        kind: InteractionIntentKind::CyclePrevious,
    });
}

fn tint_targets(
    mut targets: Query<(&mut Sprite, Option<&InteractionFocusedBy>), With<DemoTargetVisual>>,
) {
    for (mut sprite, focused_by) in &mut targets {
        sprite.color = if focused_by
            .is_some_and(|focus: &InteractionFocusedBy| !focus.interactors.is_empty())
        {
            Color::srgb(0.92, 0.72, 0.24)
        } else {
            Color::srgb(0.30, 0.34, 0.42)
        };
    }
}

fn record_prompt_messages(mut log: ResMut<DemoLog>, mut reader: MessageReader<InteractionOffered>) {
    for event in reader.read() {
        if let Some(offer) = &event.offer {
            let availability = offer
                .availability
                .as_ref()
                .map(|reason| format!("{reason:?}"))
                .unwrap_or_else(|| "available".to_owned());
            log.last_prompt = format!("{} [{}]", offer.prompt.action_label_key, availability);
        }
    }
}

fn record_completed_messages(
    mut log: ResMut<DemoLog>,
    mut reader: MessageReader<InteractionCompleted>,
) {
    for event in reader.read() {
        log.completed += 1;
        log.last_result = format!("completed: {}", event.slot_id.0);
    }
}

fn record_failed_messages(mut log: ResMut<DemoLog>, mut reader: MessageReader<InteractionFailed>) {
    for event in reader.read() {
        log.failed += 1;
        log.last_result = format!("failed: {:?}", event.reason);
    }
}

fn record_canceled_messages(
    mut log: ResMut<DemoLog>,
    mut reader: MessageReader<saddle_interaction::InteractionCanceled>,
) {
    for event in reader.read() {
        log.canceled += 1;
        log.last_result = format!("canceled: {:?}", event.reason);
    }
}

fn record_progress_messages(
    mut log: ResMut<DemoLog>,
    mut reader: MessageReader<InteractionProgress>,
) {
    for event in reader.read() {
        log.progress = event.progress;
    }
}

fn record_stage_messages(
    mut log: ResMut<DemoLog>,
    mut reader: MessageReader<InteractionStageAdvanced>,
) {
    for _ in reader.read() {
        log.stage_advanced += 1;
    }
}

fn gate_example_unlocks(
    mode: Res<DemoModeResource>,
    mut commands: Commands,
    mut reader: MessageReader<InteractionCompleted>,
    interactors: Query<&InteractionTags, With<DemoInteractor>>,
) {
    if !matches!(mode.0, DemoMode::Gated | DemoMode::VehicleBay) {
        return;
    }

    for event in reader.read() {
        if event.slot_id.0 == "power_on" {
            let mut tags = interactors
                .get(event.interactor)
                .cloned()
                .unwrap_or_default();
            if !tags.contains(&InteractionTag::from("powered")) {
                tags.tags.push(InteractionTag::from("powered"));
            }
            commands.entity(event.interactor).insert(tags);
        }

        if mode.0 == DemoMode::VehicleBay {
            let mut tags = interactors
                .get(event.interactor)
                .cloned()
                .unwrap_or_default();
            let seated = InteractionTag::from("seated");
            match event.slot_id.0.as_str() {
                "enter_vehicle" => {
                    if !tags.contains(&seated) {
                        tags.tags.push(seated);
                    }
                    commands
                        .entity(event.interactor)
                        .insert(Transform::from_xyz(120.0, 70.0, 2.0))
                        .insert(tags);
                }
                "exit_vehicle" => {
                    tags.tags.retain(|tag| tag != &seated);
                    commands
                        .entity(event.interactor)
                        .insert(Transform::from_xyz(-260.0, 0.0, 2.0))
                        .insert(tags);
                }
                _ => {}
            }
        }
    }
}

fn update_overlay(
    mode: Res<DemoModeResource>,
    log: Res<DemoLog>,
    pane: Res<InteractionDemoPane>,
    focused: Query<&InteractionPromptState, With<DemoInteractor>>,
    active: Query<&ActiveInteraction, With<DemoInteractor>>,
    interactor_tags: Query<&InteractionTags, With<DemoInteractor>>,
    mut overlay: Single<&mut Text, With<DemoOverlay>>,
) {
    let prompt = focused
        .iter()
        .next()
        .and_then(|state| state.offer.as_ref())
        .map(|offer| offer.prompt.action_label_key.clone())
        .unwrap_or_else(|| "none".to_owned());
    let active = active
        .iter()
        .next()
        .map(|active| format!("{} {:.0}%", active.slot_id.0, active.progress * 100.0))
        .unwrap_or_else(|| "none".to_owned());
    let tags = interactor_tags
        .iter()
        .next()
        .map(|tags| {
            if tags.tags.is_empty() {
                "none".to_owned()
            } else {
                tags.tags
                    .iter()
                    .map(|tag| tag.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        })
        .unwrap_or_else(|| "none".to_owned());

    **overlay = Text::new(format!(
        "{}\n{}\n\nfocused prompt: {}\nrecorded prompt: {}\nactive: {}\nprogress: {:.0}%\nactor tags: {}\ncompleted: {} failed: {} canceled: {} stage advances: {}\nlast result: {}\n\nPane:\n  range {:.1} | radius x{:.2} | hold x{:.2} | toggle {} | auto {}\n\nControls:\n  E: interact\n  Tab: next slot\n  Q: previous slot\n  Esc: cancel",
        mode.0.title(),
        mode.0.subtitle(),
        prompt,
        log.last_prompt,
        active,
        log.progress * 100.0,
        tags,
        log.completed,
        log.failed,
        log.canceled,
        log.stage_advanced,
        log.last_result,
        pane.actor_range,
        pane.detection_radius_scale,
        pane.hold_time_scale,
        pane.hold_to_toggle,
        pane.auto_interact_on_focus,
    ));
}
