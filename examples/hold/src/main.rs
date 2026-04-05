//! # Hold Interaction Example
//!
//! Demonstrates a **hold-to-complete** interaction. The player must hold **E**
//! until the progress bar reaches 100%. Releasing early cancels the attempt.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::{Cancel as InputCancel, *};
use saddle_interaction::{
    ActiveInteraction, Interactable, InteractionBehavior, InteractionCompleted,
    InteractionExecution, InteractionFailed, InteractionFocusedBy, InteractionIntent,
    InteractionIntentKind, InteractionOffered, InteractionPlugin, InteractionProgress,
    InteractionPromptState, InteractionSlot, InteractionTarget, Interactor,
};
use saddle_interaction_example_common::{
    DemoBaseTargetSlots, DemoInteractor, InteractionDemoPane, install_demo_pane,
};

// ---------------------------------------------------------------------------
// Input actions
// ---------------------------------------------------------------------------

#[derive(InputAction)]
#[action_output(bool)]
struct InteractAction;

#[derive(InputAction)]
#[action_output(bool)]
struct CancelAction;

// ---------------------------------------------------------------------------
// Components & resources
// ---------------------------------------------------------------------------

#[derive(Component)]
struct InteractorContext;

#[derive(Component)]
struct TargetVisual;

#[derive(Component)]
struct Overlay;

#[derive(Resource, Default)]
struct Log {
    last_prompt: String,
    last_result: String,
    progress: f32,
    completed: usize,
    failed: usize,
    canceled: usize,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let mut app = App::new();

    app.insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.08)));
    app.insert_resource(Log::default());

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "interaction/hold".into(),
            resolution: (1100, 720).into(),
            ..default()
        }),
        ..default()
    }));

    install_demo_pane(&mut app, false);
    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<InteractorContext>();

    // Default interaction config (hold behaviour is per-slot, not global)
    app.add_plugins(InteractionPlugin::default());

    app.add_observer(on_interact_start);
    app.add_observer(on_interact_release);
    app.add_observer(on_interact_cancel);
    app.add_observer(on_explicit_cancel);

    app.add_systems(Startup, setup_scene);
    app.add_systems(
        Update,
        (
            tint_targets,
            update_overlay,
            record_prompt,
            record_completed,
            record_failed,
            record_canceled,
            record_progress,
        ),
    );

    app.run();
}

// ---------------------------------------------------------------------------
// Scene setup
// ---------------------------------------------------------------------------

fn setup_scene(mut commands: Commands) {
    commands.spawn((Name::new("Demo Camera"), Camera2d));
    commands.spawn((
        Name::new("Backdrop"),
        Sprite::from_color(Color::srgb(0.07, 0.08, 0.11), Vec2::new(2200.0, 1600.0)),
        Transform::from_xyz(0.0, 0.0, -20.0),
    ));

    // Interactor
    commands.spawn((
        Name::new("Interactor"),
        DemoInteractor,
        InteractorContext,
        Interactor {
            max_distance: Some(500.0),
            proximity_radius: Some(500.0),
            ..default()
        },
        Sprite {
            color: Color::srgb(0.16, 0.78, 0.96),
            custom_size: Some(Vec2::new(54.0, 54.0)),
            ..default()
        },
        Transform::from_xyz(-260.0, 0.0, 2.0),
        GlobalTransform::from_xyz(-260.0, 0.0, 2.0),
        actions!(InteractorContext[
            (Action::<InteractAction>::new(), bindings![KeyCode::KeyE, GamepadButton::South]),
            (Action::<CancelAction>::new(), bindings![KeyCode::Escape, GamepadButton::East]),
        ]),
    ));

    // Target: hold for 1.2 seconds to breach
    let slots = vec![InteractionSlot {
        behavior: InteractionBehavior::Single(InteractionExecution::Hold {
            duration_seconds: 1.2,
        }),
        ..InteractionSlot::instant("breach", "Hold Breach")
    }];
    commands.spawn((
        Name::new("Bulkhead"),
        TargetVisual,
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
        Sprite {
            color: Color::srgb(0.30, 0.34, 0.42),
            custom_size: Some(Vec2::new(92.0, 92.0)),
            ..default()
        },
        Transform::from_xyz(110.0, 0.0, 1.0),
        GlobalTransform::from_xyz(110.0, 0.0, 1.0),
    ));

    // Overlay
    commands.spawn((
        Name::new("Overlay"),
        Overlay,
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
}

// ---------------------------------------------------------------------------
// Input → intent wiring
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Visuals
// ---------------------------------------------------------------------------

fn tint_targets(
    mut targets: Query<(&mut Sprite, Option<&InteractionFocusedBy>), With<TargetVisual>>,
) {
    for (mut sprite, focused_by) in &mut targets {
        sprite.color =
            if focused_by.is_some_and(|f: &InteractionFocusedBy| !f.interactors.is_empty()) {
                Color::srgb(0.92, 0.72, 0.24)
            } else {
                Color::srgb(0.30, 0.34, 0.42)
            };
    }
}

// ---------------------------------------------------------------------------
// Lifecycle recording
// ---------------------------------------------------------------------------

fn record_prompt(mut log: ResMut<Log>, mut reader: MessageReader<InteractionOffered>) {
    for event in reader.read() {
        if let Some(offer) = &event.offer {
            log.last_prompt = offer.prompt.action_label_key.clone();
        }
    }
}

fn record_completed(mut log: ResMut<Log>, mut reader: MessageReader<InteractionCompleted>) {
    for event in reader.read() {
        log.completed += 1;
        log.last_result = format!("completed: {}", event.slot_id.0);
    }
}

fn record_failed(mut log: ResMut<Log>, mut reader: MessageReader<InteractionFailed>) {
    for event in reader.read() {
        log.failed += 1;
        log.last_result = format!("failed: {:?}", event.reason);
    }
}

fn record_canceled(
    mut log: ResMut<Log>,
    mut reader: MessageReader<saddle_interaction::InteractionCanceled>,
) {
    for event in reader.read() {
        log.canceled += 1;
        log.last_result = format!("canceled: {:?}", event.reason);
    }
}

fn record_progress(mut log: ResMut<Log>, mut reader: MessageReader<InteractionProgress>) {
    for event in reader.read() {
        log.progress = event.progress;
    }
}

// ---------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------

fn update_overlay(
    log: Res<Log>,
    pane: Res<InteractionDemoPane>,
    focused: Query<&InteractionPromptState, With<DemoInteractor>>,
    active: Query<&ActiveInteraction, With<DemoInteractor>>,
    mut overlay: Single<&mut Text, With<Overlay>>,
) {
    let prompt = focused
        .iter()
        .next()
        .and_then(|s| s.offer.as_ref())
        .map(|o| o.prompt.action_label_key.clone())
        .unwrap_or_else(|| "none".to_owned());
    let active_str = active
        .iter()
        .next()
        .map(|a| format!("{} {:.0}%", a.slot_id.0, a.progress * 100.0))
        .unwrap_or_else(|| "none".to_owned());

    **overlay = Text::new(format!(
        "interaction/hold\n\
         Hold E until the progress reaches 100%.\n\n\
         focused prompt: {prompt}\n\
         recorded prompt: {}\n\
         active: {active_str}\n\
         progress: {:.0}%\n\
         completed: {} failed: {} canceled: {}\n\
         last result: {}\n\n\
         Pane: range {:.1} | hold x{:.2}\n\n\
         Controls:\n  E (hold): interact\n  Esc: cancel",
        log.last_prompt,
        log.progress * 100.0,
        log.completed,
        log.failed,
        log.canceled,
        log.last_result,
        pane.actor_range,
        pane.hold_time_scale,
    ));
}
