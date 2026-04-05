use bevy::{prelude::*, ui::Val2};
use bevy_enhanced_input::prelude::{Cancel as InputCancel, *};
use saddle_animation_text_animation::{
    TextAnimationAction, TextAnimationBundle, TextAnimationCommand, TextAnimationCompleted,
    TextAnimationConfig, TextAnimationMarkup, TextAnimationPlugin, TextRevealSound,
    TextRevealSoundRequested,
};
use saddle_animation_tween::{
    Tween, TweenBundle, TweenCompleted, TweenPlayer, TweenPlugin, TweenProgressed,
    background_opacity, scale as transform_scale, text_opacity, ui_translation_px,
};
use saddle_interaction::{
    Interactable, InteractionBehavior, InteractionCompleted, InteractionConfig, InteractionIntent,
    InteractionIntentKind, InteractionOffered, InteractionPlugin, InteractionProgress,
    InteractionReservationPolicy, InteractionSlot, InteractionTarget, Interactor,
};
use saddle_interaction_example_common::pane_plugins;
use saddle_pane::prelude::*;

const PILOT_START: Vec3 = Vec3::new(-260.0, -32.0, 4.0);
const TERMINAL_POSITION: Vec3 = Vec3::new(150.0, -22.0, 4.0);
const TERMINAL_GLOW_POSITION: Vec3 = Vec3::new(150.0, 52.0, 3.0);
const PANEL_HIDDEN_OFFSET: Vec2 = Vec2::new(-430.0, 0.0);
const PANEL_OPEN_OFFSET: Vec2 = Vec2::ZERO;
const ROOM_MIN: Vec2 = Vec2::new(-360.0, -170.0);
const ROOM_MAX: Vec2 = Vec2::new(230.0, 120.0);

const SLOT_UPLINK: &str = "uplink_feed";
const SLOT_DOCKING: &str = "request_docking";
const SLOT_STOW: &str = "stow_panel";

const UPLINK_LINES: &[(&str, &str)] = &[
    (
        "Uplink Warmup",
        "<wave>Hangar uplink online</wave>. Cargo armatures are <scale>green across all lanes</scale>.",
    ),
    (
        "Relay Sweep",
        "Maintenance: <wave>shield lattice stable</wave>. Route the tug train to <shake>berth three</shake>.",
    ),
];

const DOCKING_LINES: &[(&str, &str)] = &[
    (
        "Docking Window",
        "Traffic Control: <shake>hold your vector</shake>. Approach <scale>lane seven</scale> on my mark.",
    ),
    (
        "Pilot Callout",
        "Chief Pilot: <wave>Beacon handshake confirmed</wave>. Swing wide and expect a <scale>manual latch</scale>.",
    ),
];

#[derive(Component)]
struct Pilot;

#[derive(Component, Default)]
struct PilotContext;

#[derive(Component, Default)]
struct PilotMotion {
    axis: Vec2,
}

#[derive(Component)]
struct Terminal;

#[derive(Component)]
struct TerminalGlow;

#[derive(Component)]
struct TerminalPanel;

#[derive(Component)]
struct DialogueBody;

#[derive(Component)]
struct DialogueFooter;

#[derive(Component)]
struct Overlay;

#[derive(Component, Clone)]
struct BaseTerminalSlots(Vec<InteractionSlot>);

#[derive(Resource)]
struct DialogueScene {
    terminal: Entity,
    glow: Entity,
    panel: Entity,
    body: Entity,
}

#[derive(Resource, Default)]
struct DialogueState {
    panel_open: bool,
    prompt_label: String,
    active_slot: String,
    last_line_title: String,
    last_tween_label: String,
    last_sound_cue: String,
    hold_progress: f32,
    voice_blips: usize,
    completed_lines: usize,
    panel_transitions: usize,
}

#[derive(Resource, Clone, Pane)]
#[pane(title = "Dialogue Terminal")]
struct DialogueTerminalPane {
    #[pane(slider, min = 50.0, max = 500.0, step = 10.0)]
    interactor_range: f32,
    #[pane(slider, min = 0.2, max = 2.5, step = 0.05)]
    uplink_hold_secs: f32,
    #[pane(slider, min = 120.0, max = 900.0, step = 10.0)]
    panel_speed_units: f32,
    #[pane(slider, min = 0.35, max = 0.98, step = 0.01)]
    panel_opacity: f32,
    #[pane(slider, min = 4.0, max = 30.0, step = 0.5)]
    reveal_units_per_second: f32,
    #[pane(slider, min = 120.0, max = 420.0, step = 5.0)]
    pilot_speed: f32,
}

impl Default for DialogueTerminalPane {
    fn default() -> Self {
        Self {
            interactor_range: 200.0,
            uplink_hold_secs: 0.9,
            panel_speed_units: 560.0,
            panel_opacity: 0.96,
            reveal_units_per_second: 14.0,
            pilot_speed: 240.0,
        }
    }
}

#[derive(InputAction)]
#[action_output(bool)]
struct InteractAction;

#[derive(InputAction)]
#[action_output(bool)]
struct NextSlotAction;

#[derive(InputAction)]
#[action_output(bool)]
struct PrevSlotAction;

#[derive(InputAction)]
#[action_output(f32)]
struct MoveXAction;

#[derive(InputAction)]
#[action_output(f32)]
struct MoveYAction;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.035, 0.045, 0.065)));
    app.insert_resource(InteractionConfig::default());
    app.init_resource::<DialogueTerminalPane>();
    app.init_resource::<DialogueState>();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "interaction dialogue_terminal".into(),
            resolution: (1320, 840).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(pane_plugins());
    app.register_pane::<DialogueTerminalPane>();
    app.add_plugins(EnhancedInputPlugin);
    app.add_plugins(InteractionPlugin::default());
    app.add_plugins(TextAnimationPlugin::default());
    app.add_plugins(TweenPlugin::default());
    app.add_input_context::<PilotContext>();
    app.add_observer(on_interact_start);
    app.add_observer(on_interact_release);
    app.add_observer(on_interact_cancel);
    app.add_observer(on_next_slot);
    app.add_observer(on_prev_slot);
    app.add_observer(on_move_x);
    app.add_observer(clear_move_x_on_cancel);
    app.add_observer(clear_move_x_on_complete);
    app.add_observer(on_move_y);
    app.add_observer(clear_move_y_on_cancel);
    app.add_observer(clear_move_y_on_complete);
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            sync_pane
                .before(saddle_interaction::InteractionSystems::Detect)
                .before(saddle_animation_text_animation::TextAnimationSystems::DetectChanges),
            move_pilot.before(saddle_interaction::InteractionSystems::Detect),
            handle_terminal_completions
                .after(saddle_interaction::InteractionSystems::Feedback)
                .before(saddle_animation_tween::TweenSystems::ResolveConflicts)
                .before(saddle_animation_text_animation::TextAnimationSystems::Advance),
            record_prompt_messages.after(saddle_interaction::InteractionSystems::Feedback),
            record_progress_messages.after(saddle_interaction::InteractionSystems::Feedback),
            record_reveal_sounds
                .after(saddle_animation_text_animation::TextAnimationSystems::Advance),
            record_text_completions
                .after(saddle_animation_text_animation::TextAnimationSystems::Advance),
            record_tween_messages.after(saddle_animation_tween::TweenSystems::Cleanup),
            tint_terminal.after(saddle_interaction::InteractionSystems::Feedback),
            update_footer,
            update_overlay,
        )
            .chain(),
    );
    app.run();
}

fn setup(mut commands: Commands, pane: Res<DialogueTerminalPane>) {
    commands.spawn((Name::new("Camera"), Camera2d));
    commands.spawn((
        Name::new("Backdrop"),
        Sprite::from_color(Color::srgb(0.05, 0.06, 0.09), Vec2::new(2200.0, 1600.0)),
        Transform::from_xyz(0.0, 0.0, -20.0),
    ));
    commands.spawn((
        Name::new("Hangar Deck"),
        Sprite::from_color(Color::srgb(0.09, 0.11, 0.14), Vec2::new(2200.0, 320.0)),
        Transform::from_xyz(0.0, -220.0, -16.0),
    ));
    commands.spawn((
        Name::new("Control Canopy"),
        Sprite::from_color(Color::srgb(0.07, 0.10, 0.15), Vec2::new(2200.0, 250.0)),
        Transform::from_xyz(0.0, 250.0, -15.0),
    ));
    for index in -6..=6 {
        commands.spawn((
            Name::new(format!("Deck Guide {}", index + 7)),
            Sprite::from_color(Color::srgba(0.86, 0.92, 1.0, 0.03), Vec2::new(4.0, 1400.0)),
            Transform::from_xyz(index as f32 * 110.0, 0.0, -14.0),
        ));
    }

    commands.spawn((
        Name::new("Freight Rover Hull"),
        Sprite::from_color(Color::srgb(0.16, 0.22, 0.29), Vec2::new(410.0, 150.0)),
        Transform::from_xyz(120.0, -70.0, 0.0),
    ));
    commands.spawn((
        Name::new("Freight Rover Canopy"),
        Sprite::from_color(Color::srgb(0.20, 0.36, 0.48), Vec2::new(220.0, 74.0)),
        Transform::from_xyz(50.0, -20.0, 1.0),
    ));
    commands.spawn((
        Name::new("Service Ramp"),
        Sprite::from_color(Color::srgb(0.18, 0.20, 0.24), Vec2::new(190.0, 46.0)),
        Transform::from_xyz(-20.0, -150.0, 0.0),
    ));

    let terminal = commands
        .spawn((
            Name::new("Comms Terminal"),
            Terminal,
            Interactable {
                focus_radius: Some(200.0),
                priority: 1.0,
                ..default()
            },
            InteractionTarget {
                slots: terminal_slots(pane.uplink_hold_secs),
            },
            BaseTerminalSlots(terminal_slots(pane.uplink_hold_secs)),
            Sprite::from_color(Color::srgb(0.22, 0.31, 0.42), Vec2::new(124.0, 168.0)),
            Transform::from_translation(TERMINAL_POSITION),
        ))
        .id();

    let glow = commands
        .spawn((
            Name::new("Terminal Glow"),
            TerminalGlow,
            Sprite::from_color(Color::srgba(0.38, 0.82, 0.98, 0.24), Vec2::new(132.0, 56.0)),
            Transform::from_translation(TERMINAL_GLOW_POSITION),
        ))
        .id();

    commands.spawn((
        Name::new("Pilot"),
        Pilot,
        PilotContext,
        PilotMotion::default(),
        Interactor {
            max_distance: Some(200.0),
            proximity_radius: Some(200.0),
            ..default()
        },
        Sprite::from_color(Color::srgb(0.95, 0.76, 0.24), Vec2::new(50.0, 62.0)),
        Transform::from_translation(PILOT_START),
        actions!(PilotContext[
            (
                Action::<InteractAction>::new(),
                bindings![KeyCode::KeyE, GamepadButton::South],
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
                Action::<MoveXAction>::new(),
                Bindings::spawn(Bidirectional::new(KeyCode::KeyA, KeyCode::KeyD)),
            ),
            (
                Action::<MoveYAction>::new(),
                Bindings::spawn(Bidirectional::new(KeyCode::KeyS, KeyCode::KeyW)),
            ),
        ]),
    ));

    let mut body = Entity::PLACEHOLDER;
    let panel = commands
        .spawn((
            Name::new("Dialogue Panel"),
            TerminalPanel,
            Node {
                position_type: PositionType::Absolute,
                right: px(42.0),
                top: px(72.0),
                width: px(470.0),
                min_height: px(316.0),
                max_height: px(700.0),
                padding: UiRect::all(px(22.0)),
                row_gap: px(14.0),
                flex_direction: FlexDirection::Column,
                border_radius: BorderRadius::all(px(26.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            UiTransform::from_translation(Val2::px(PANEL_HIDDEN_OFFSET.x, PANEL_HIDDEN_OFFSET.y)),
            BackgroundColor(Color::srgba(0.06, 0.10, 0.15, 0.0)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Name::new("Panel Eyebrow"),
                Text::new("Hangar Control // Terminal Feed"),
                TextFont {
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.87, 1.0)),
            ));
            body = panel
                .spawn((
                    Name::new("Panel Body"),
                    DialogueBody,
                    Text::new(""),
                    TextFont {
                        font_size: 29.0,
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
                    TextAnimationMarkup::single(""),
                    TextRevealSound {
                        cue_id: "dialogue.terminal.blip".into(),
                        ..default()
                    },
                    TextAnimationBundle {
                        config: TextAnimationConfig::typewriter(pane.reveal_units_per_second),
                        ..default()
                    },
                ))
                .id();
            panel.spawn((
                Name::new("Panel Footer"),
                DialogueFooter,
                Text::new("cycle a terminal slot to queue a transmission"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.83, 0.90)),
            ));
        })
        .id();

    commands.spawn((
        Name::new("Overlay"),
        Overlay,
        Text::new(String::new()),
        Node {
            position_type: PositionType::Absolute,
            left: px(20.0),
            top: px(18.0),
            width: px(440.0),
            padding: UiRect::all(px(16.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.84)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));

    commands.insert_resource(DialogueScene {
        terminal,
        glow,
        panel,
        body,
    });
}

fn terminal_slots(uplink_hold_secs: f32) -> Vec<InteractionSlot> {
    let mut uplink = InteractionSlot::instant(SLOT_UPLINK, "Link uplink");
    uplink.behavior = InteractionBehavior::Single(saddle_interaction::InteractionExecution::Hold {
        duration_seconds: uplink_hold_secs,
    });
    uplink.priority = 2.0;
    uplink.reservation = InteractionReservationPolicy::Exclusive;

    let mut docking = InteractionSlot::instant(SLOT_DOCKING, "Request docking");
    docking.priority = 1.2;

    let mut stow = InteractionSlot::instant(SLOT_STOW, "Stow panel");
    stow.priority = 0.6;

    vec![uplink, docking, stow]
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

fn on_move_x(trigger: On<Fire<MoveXAction>>, mut pilots: Query<&mut PilotMotion, With<Pilot>>) {
    if let Ok(mut motion) = pilots.get_mut(trigger.context) {
        motion.axis.x = trigger.value;
    }
}

fn clear_move_x_on_cancel(
    trigger: On<InputCancel<MoveXAction>>,
    mut pilots: Query<&mut PilotMotion, With<Pilot>>,
) {
    if let Ok(mut motion) = pilots.get_mut(trigger.context) {
        motion.axis.x = 0.0;
    }
}

fn clear_move_x_on_complete(
    trigger: On<Complete<MoveXAction>>,
    mut pilots: Query<&mut PilotMotion, With<Pilot>>,
) {
    if let Ok(mut motion) = pilots.get_mut(trigger.context) {
        motion.axis.x = 0.0;
    }
}

fn on_move_y(trigger: On<Fire<MoveYAction>>, mut pilots: Query<&mut PilotMotion, With<Pilot>>) {
    if let Ok(mut motion) = pilots.get_mut(trigger.context) {
        motion.axis.y = -trigger.value;
    }
}

fn clear_move_y_on_cancel(
    trigger: On<InputCancel<MoveYAction>>,
    mut pilots: Query<&mut PilotMotion, With<Pilot>>,
) {
    if let Ok(mut motion) = pilots.get_mut(trigger.context) {
        motion.axis.y = 0.0;
    }
}

fn clear_move_y_on_complete(
    trigger: On<Complete<MoveYAction>>,
    mut pilots: Query<&mut PilotMotion, With<Pilot>>,
) {
    if let Ok(mut motion) = pilots.get_mut(trigger.context) {
        motion.axis.y = 0.0;
    }
}

fn sync_pane(
    pane: Res<DialogueTerminalPane>,
    scene: Res<DialogueScene>,
    state: Res<DialogueState>,
    mut interactors: Query<&mut Interactor, With<Pilot>>,
    mut terminals: Query<
        (
            &mut Interactable,
            &mut InteractionTarget,
            &BaseTerminalSlots,
        ),
        With<Terminal>,
    >,
    mut text: Query<&mut TextAnimationConfig, With<DialogueBody>>,
    mut panel: Query<&mut BackgroundColor, With<TerminalPanel>>,
) {
    if !pane.is_changed() {
        return;
    }

    for mut interactor in &mut interactors {
        interactor.max_distance = Some(pane.interactor_range);
        interactor.proximity_radius = Some(pane.interactor_range);
    }

    for (mut interactable, mut target, base) in &mut terminals {
        interactable.focus_radius = Some(pane.interactor_range);
        target.slots = base.0.clone();
        for slot in &mut target.slots {
            if slot.id.0 == SLOT_UPLINK {
                slot.behavior =
                    InteractionBehavior::Single(saddle_interaction::InteractionExecution::Hold {
                        duration_seconds: pane.uplink_hold_secs,
                    });
            }
        }
    }

    if let Ok(mut config) = text.get_mut(scene.body) {
        config.typewriter.units_per_second = pane.reveal_units_per_second;
    }

    if state.panel_open
        && let Ok(mut background) = panel.get_mut(scene.panel)
    {
        background.0 = Color::srgba(0.06, 0.10, 0.15, pane.panel_opacity);
    }
}

fn move_pilot(
    time: Res<Time>,
    pane: Res<DialogueTerminalPane>,
    mut pilots: Query<(&PilotMotion, &mut Transform), With<Pilot>>,
) {
    for (motion, mut transform) in &mut pilots {
        let axis = motion.axis.clamp_length_max(1.0);
        transform.translation += axis.extend(0.0) * pane.pilot_speed * time.delta_secs();
        transform.translation.x = transform.translation.x.clamp(ROOM_MIN.x, ROOM_MAX.x);
        transform.translation.y = transform.translation.y.clamp(ROOM_MIN.y, ROOM_MAX.y);
    }
}

fn handle_terminal_completions(
    mut commands: Commands,
    pane: Res<DialogueTerminalPane>,
    scene: Res<DialogueScene>,
    mut state: ResMut<DialogueState>,
    mut completed: MessageReader<InteractionCompleted>,
    mut markup: Query<&mut TextAnimationMarkup, With<DialogueBody>>,
    mut text_commands: MessageWriter<TextAnimationCommand>,
) {
    for event in completed.read() {
        if event.target != scene.terminal {
            continue;
        }

        state.active_slot = event.slot_id.0.clone();
        match event.slot_id.0.as_str() {
            SLOT_UPLINK => {
                let (title, body) = UPLINK_LINES[state.completed_lines % UPLINK_LINES.len()];
                state.completed_lines += 1;
                state.last_line_title = title.to_owned();
                state.panel_open = true;
                state.hold_progress = 0.0;
                state.voice_blips = 0;
                apply_dialogue_markup(&mut markup, scene.body, body);
                text_commands.write(TextAnimationCommand {
                    entity: scene.body,
                    action: TextAnimationAction::Restart,
                });
                spawn_panel_tween(&mut commands, &pane, &scene, true);
                spawn_terminal_pulse(&mut commands, scene.glow);
            }
            SLOT_DOCKING => {
                let (title, body) = DOCKING_LINES[state.completed_lines % DOCKING_LINES.len()];
                state.completed_lines += 1;
                state.last_line_title = title.to_owned();
                state.panel_open = true;
                state.hold_progress = 0.0;
                state.voice_blips = 0;
                apply_dialogue_markup(&mut markup, scene.body, body);
                text_commands.write(TextAnimationCommand {
                    entity: scene.body,
                    action: TextAnimationAction::Restart,
                });
                spawn_panel_tween(&mut commands, &pane, &scene, true);
                spawn_terminal_pulse(&mut commands, scene.glow);
            }
            SLOT_STOW => {
                state.last_line_title = "Panel Stowed".into();
                state.panel_open = false;
                state.hold_progress = 0.0;
                text_commands.write(TextAnimationCommand {
                    entity: scene.body,
                    action: TextAnimationAction::Pause,
                });
                spawn_panel_tween(&mut commands, &pane, &scene, false);
            }
            _ => {}
        }
    }
}

fn apply_dialogue_markup(
    markup: &mut Query<&mut TextAnimationMarkup, With<DialogueBody>>,
    body_entity: Entity,
    value: &str,
) {
    if let Ok(mut body) = markup.get_mut(body_entity) {
        body.sections = vec![value.to_owned()];
    }
}

fn spawn_panel_tween(
    commands: &mut Commands,
    pane: &DialogueTerminalPane,
    scene: &DialogueScene,
    open: bool,
) {
    let tween = Tween::parallel([
        ui_translation_px(scene.panel)
            .from_current()
            .to(if open {
                PANEL_OPEN_OFFSET
            } else {
                PANEL_HIDDEN_OFFSET
            })
            .units_per_second(pane.panel_speed_units)
            .ease(if open {
                EaseFunction::BackOut
            } else {
                EaseFunction::SineIn
            })
            .build(),
        background_opacity(scene.panel)
            .from_current()
            .to(if open { pane.panel_opacity } else { 0.0 })
            .duration_secs(0.28)
            .ease(EaseFunction::SineOut)
            .build(),
        text_opacity(scene.body)
            .from_current()
            .to(if open { 1.0 } else { 0.0 })
            .duration_secs(0.24)
            .ease(EaseFunction::SineOut)
            .build(),
    ]);

    commands.spawn(TweenBundle::new(TweenPlayer::new(tween).with_label(
        if open {
            "dialogue panel open"
        } else {
            "dialogue panel close"
        },
    )));
}

fn spawn_terminal_pulse(commands: &mut Commands, glow: Entity) {
    let tween = Tween::sequence([
        Tween::parallel([transform_scale(glow)
            .from_current()
            .to(Vec3::splat(1.16))
            .duration_secs(0.12)
            .ease(EaseFunction::SineOut)
            .build()]),
        Tween::parallel([transform_scale(glow)
            .from_current()
            .to(Vec3::ONE)
            .duration_secs(0.18)
            .ease(EaseFunction::SineInOut)
            .build()]),
    ]);

    commands.spawn(TweenBundle::new(
        TweenPlayer::new(tween).with_label("terminal pulse"),
    ));
}

fn record_prompt_messages(
    mut state: ResMut<DialogueState>,
    mut offered: MessageReader<InteractionOffered>,
) {
    for event in offered.read() {
        state.prompt_label = event
            .offer
            .as_ref()
            .map(|offer| offer.prompt.action_label_key.clone())
            .unwrap_or_else(|| "step into the comms cone".into());
        state.active_slot = event
            .offer
            .as_ref()
            .map(|offer| offer.slot_id.0.clone())
            .unwrap_or_default();
    }
}

fn record_progress_messages(
    mut state: ResMut<DialogueState>,
    mut progress: MessageReader<InteractionProgress>,
) {
    for event in progress.read() {
        state.hold_progress = event.progress;
    }
}

fn record_reveal_sounds(
    scene: Res<DialogueScene>,
    mut state: ResMut<DialogueState>,
    mut sounds: MessageReader<TextRevealSoundRequested>,
) {
    for event in sounds.read() {
        if event.entity != scene.body {
            continue;
        }

        state.voice_blips += 1;
        state.last_sound_cue = event.cue_id.clone();
    }
}

fn record_text_completions(
    scene: Res<DialogueScene>,
    mut state: ResMut<DialogueState>,
    mut completed: MessageReader<TextAnimationCompleted>,
) {
    for event in completed.read() {
        if event.entity == scene.body {
            state.hold_progress = 0.0;
        }
    }
}

fn record_tween_messages(
    scene: Res<DialogueScene>,
    mut state: ResMut<DialogueState>,
    mut progress: MessageReader<TweenProgressed>,
    mut completed: MessageReader<TweenCompleted>,
) {
    for event in progress.read() {
        if event.primary_target == Some(scene.panel) {
            state.last_tween_label = event.label.clone().unwrap_or_default();
        }
    }

    for event in completed.read() {
        if event.primary_target == Some(scene.panel) {
            state.panel_transitions += 1;
            state.last_tween_label = event.label.clone().unwrap_or_default();
        }
    }
}

fn tint_terminal(
    mut terminals: Query<&mut Sprite, (With<Terminal>, Without<TerminalGlow>)>,
    mut glow: Query<&mut Sprite, (With<TerminalGlow>, Without<Terminal>)>,
    focused: Query<&saddle_interaction::InteractionFocusedBy, With<Terminal>>,
) {
    let active = focused
        .iter()
        .next()
        .is_some_and(|entry| !entry.interactors.is_empty());

    if let Ok(mut sprite) = terminals.single_mut() {
        sprite.color = if active {
            Color::srgb(0.30, 0.48, 0.62)
        } else {
            Color::srgb(0.22, 0.31, 0.42)
        };
    }

    if let Ok(mut sprite) = glow.single_mut() {
        sprite.color = if active {
            Color::srgba(0.45, 0.88, 1.0, 0.34)
        } else {
            Color::srgba(0.38, 0.82, 0.98, 0.24)
        };
    }
}

fn update_footer(state: Res<DialogueState>, mut footer: Single<&mut Text, With<DialogueFooter>>) {
    **footer = Text::new(format!(
        "slot: {}  |  voice pips: {}  |  panel tween: {}",
        display_field(&state.active_slot),
        state.voice_blips,
        display_field(&state.last_tween_label),
    ));
}

fn update_overlay(
    pane: Res<DialogueTerminalPane>,
    state: Res<DialogueState>,
    mut overlay: Single<&mut Text, With<Overlay>>,
) {
    **overlay = Text::new(format!(
        "Dialogue terminal integration\n\
         WASD move pilot\n\
         E interact  |  Tab / Q cycle terminal slots\n\n\
         prompt: {}\n\
         active slot: {}\n\
         last line: {}\n\
         hold progress: {:.0}%\n\
         panel open: {}\n\
         last sound cue: {}\n\
         panel transitions: {}\n\n\
         pane sync\n\
         range {:.1}  |  hold {:.2}s  |  tween {:.0} u/s  |  reveal {:.1} u/s",
        display_field(&state.prompt_label),
        display_field(&state.active_slot),
        display_field(&state.last_line_title),
        state.hold_progress * 100.0,
        if state.panel_open { "yes" } else { "no" },
        display_field(&state.last_sound_cue),
        state.panel_transitions,
        pane.interactor_range,
        pane.uplink_hold_secs,
        pane.panel_speed_units,
        pane.reveal_units_per_second,
    ));
}

fn display_field(value: &str) -> &str {
    if value.is_empty() { "waiting" } else { value }
}
