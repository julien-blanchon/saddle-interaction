//! # Vehicle Bay Interaction Example
//!
//! Demonstrates **exclusive reservations** and **tag gating** for a vehicle
//! enter/exit flow. The "Enter Rover" slot is blocked when the actor already
//! has the `"seated"` tag, and the "Exit Hatch" slot requires it. Completing
//! "enter_vehicle" teleports the actor into the cockpit and adds `"seated"`;
//! completing "exit_vehicle" removes it and teleports back.

use bevy::prelude::*;
use bevy_enhanced_input::prelude::{Cancel as InputCancel, *};
use saddle_interaction::{
    ActiveInteraction, Interactable, InteractionAvailabilityConfig, InteractionCompleted,
    InteractionFailed, InteractionFocusedBy, InteractionIntent,
    InteractionIntentKind, InteractionOffered, InteractionPlugin, InteractionProgress,
    InteractionPromptState, InteractionReservationPolicy, InteractionSlot, InteractionTag,
    InteractionTags, InteractionTarget, Interactor,
};
use saddle_interaction_example_common::{
    DemoBaseTargetSlots, DemoInteractor, install_demo_pane, InteractionDemoPane,
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
            title: "interaction/vehicle_bay".into(),
            resolution: (1100, 720).into(),
            ..default()
        }),
        ..default()
    }));

    install_demo_pane(&mut app, false);
    app.add_plugins(EnhancedInputPlugin);
    app.add_input_context::<InteractorContext>();
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
            record_progress,
            handle_vehicle_events,
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

    // Rover body (visual only)
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

    // Interactor (starts without the "seated" tag)
    commands.spawn((
        Name::new("Interactor"),
        DemoInteractor,
        InteractorContext,
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
        actions!(InteractorContext[
            (Action::<InteractAction>::new(), bindings![KeyCode::KeyE, GamepadButton::South]),
            (Action::<CancelAction>::new(), bindings![KeyCode::Escape, GamepadButton::East]),
        ]),
    ));

    // Cockpit: enter only when NOT seated (exclusive reservation)
    let cockpit_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            blocked_actor_tags: vec![InteractionTag::from("seated")],
            ..default()
        },
        reservation: InteractionReservationPolicy::Exclusive,
        ..InteractionSlot::instant("enter_vehicle", "Enter Rover")
    }];
    commands.spawn((
        Name::new("Cockpit"),
        TargetVisual,
        DemoBaseTargetSlots(cockpit_slots.clone()),
        Interactable::default(),
        InteractionTarget {
            slots: cockpit_slots,
        },
        Sprite {
            color: Color::srgb(0.30, 0.34, 0.42),
            custom_size: Some(Vec2::new(92.0, 92.0)),
            ..default()
        },
        Transform::from_xyz(120.0, 70.0, 1.0),
        GlobalTransform::from_xyz(120.0, 70.0, 1.0),
    ));

    // Exit Hatch: available only when seated (exclusive reservation)
    let exit_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            required_actor_tags: vec![InteractionTag::from("seated")],
            ..default()
        },
        reservation: InteractionReservationPolicy::Exclusive,
        ..InteractionSlot::instant("exit_vehicle", "Exit Rover")
    }];
    commands.spawn((
        Name::new("Exit Hatch"),
        TargetVisual,
        DemoBaseTargetSlots(exit_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: exit_slots },
        Sprite {
            color: Color::srgb(0.30, 0.34, 0.42),
            custom_size: Some(Vec2::new(92.0, 92.0)),
            ..default()
        },
        Transform::from_xyz(120.0, -70.0, 1.0),
        GlobalTransform::from_xyz(120.0, -70.0, 1.0),
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
// Vehicle enter/exit logic: toggle "seated" tag and teleport actor
// ---------------------------------------------------------------------------

fn handle_vehicle_events(
    mut commands: Commands,
    mut reader: MessageReader<InteractionCompleted>,
    interactors: Query<&InteractionTags, With<DemoInteractor>>,
) {
    for event in reader.read() {
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
            let availability = offer
                .availability
                .as_ref()
                .map(|r| format!("{r:?}"))
                .unwrap_or_else(|| "available".to_owned());
            log.last_prompt = format!("{} [{}]", offer.prompt.action_label_key, availability);
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
    interactor_tags: Query<&InteractionTags, With<DemoInteractor>>,
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
    let tags = interactor_tags
        .iter()
        .next()
        .map(|t| {
            if t.tags.is_empty() {
                "none".to_owned()
            } else {
                t.tags
                    .iter()
                    .map(|tag| tag.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        })
        .unwrap_or_else(|| "none".to_owned());

    **overlay = Text::new(format!(
        "interaction/vehicle_bay\n\
         Enter the rover, occupy the exclusive seat, then exit.\n\n\
         focused prompt: {prompt}\n\
         recorded prompt: {}\n\
         active: {active_str}\n\
         progress: {:.0}%\n\
         actor tags: {tags}\n\
         completed: {} failed: {}\n\
         last result: {}\n\n\
         Pane: range {:.1}\n\n\
         Controls:\n  E: interact\n  Esc: cancel",
        log.last_prompt,
        log.progress * 100.0,
        log.completed,
        log.failed,
        log.last_result,
        pane.actor_range,
    ));
}
