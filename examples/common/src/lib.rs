use bevy::prelude::*;
use bevy_enhanced_input::prelude::{Cancel as InputCancel, *};
use saddle_interaction::{
    ActiveInteraction, Interactable, InteractionBehavior, InteractionCompleted, InteractionConfig,
    InteractionFailed, InteractionFocusedBy, InteractionIntent, InteractionIntentKind,
    InteractionOffered, InteractionPlugin, InteractionProgress, InteractionSlot, InteractionStage,
    InteractionStageAdvanced, InteractionTag, InteractionTags, InteractionTarget, Interactor,
};

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
struct DemoInteractor;

#[derive(Component)]
struct DemoTargetVisual;

#[derive(Component)]
struct DemoOverlay;

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
    if mode.0 != DemoMode::Gated {
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
    }
}

fn update_overlay(
    mode: Res<DemoModeResource>,
    log: Res<DemoLog>,
    focused: Query<&saddle_interaction::InteractionPromptState, With<DemoInteractor>>,
    active: Query<&ActiveInteraction, With<DemoInteractor>>,
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

    **overlay = Text::new(format!(
        "{}\n{}\n\nfocused prompt: {}\nrecorded prompt: {}\nactive: {}\nprogress: {:.0}%\nlast result: {}\ncompleted: {} failed: {} canceled: {} stage advances: {}\n\nControls:\n  E: interact\n  Tab: next slot\n  Q: previous slot\n  Esc: cancel",
        mode.0.title(),
        mode.0.subtitle(),
        prompt,
        log.last_prompt,
        active,
        log.progress * 100.0,
        log.last_result,
        log.completed,
        log.failed,
        log.canceled,
        log.stage_advanced,
    ));
}
