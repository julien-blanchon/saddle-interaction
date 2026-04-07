//! # Interaction Lab
//!
//! Comprehensive showcase combining all interaction features in a single 3D
//! room. Number keys 1–6 teleport the player between stations. Includes BRP
//! inspection (dev) and E2E scenarios (e2e feature).
//!
//! Stations:
//! 1. **Instant** — Chest (Open)
//! 2. **Hold** — Valve (Turn, 0.75s)
//! 3. **Multi-slot** — Terminal (Hack / Read)
//! 4. **Sequence** — Lever (Prime → Pull → Reset, loops)
//! 5. **Gated** — Generator + Door (tag-gated)
//! 6. **Vehicle** — Cockpit + Exit Hatch (exclusive reservation)

#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use bevy::prelude::*;
#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
use bevy_brp_extras::BrpExtrasPlugin;
use saddle_interaction::{
    ActiveInteraction, FocusedInteraction, Interactable, InteractionAvailabilityConfig,
    InteractionBehavior, InteractionCanceled, InteractionCompleted, InteractionConfig,
    InteractionDebugSettings, InteractionExecution, InteractionFailed, InteractionIntent,
    InteractionIntentKind, InteractionPrompt, InteractionPromptState,
    InteractionReservationPolicy, InteractionSlot, InteractionStage, InteractionStageAdvanced,
    InteractionTag, InteractionTags, InteractionTarget, Interactor, SequenceAdvanceMode,
};
use saddle_interaction_example_common as common;
use common::{DemoBaseTargetSlots, DemoPlayer, DemoPlayerController};

// ---------------------------------------------------------------------------
// Station layout (3D positions)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LabStation {
    Instant,
    Hold,
    Multi,
    Sequence,
    Gated,
    Vehicle,
}

struct StationProfile {
    player_pos: Vec3,
    look_at: Vec3,
    range: f32,
}

fn station_profile(station: LabStation) -> StationProfile {
    match station {
        LabStation::Instant => StationProfile {
            player_pos: Vec3::new(-8.0, 1.6, 5.0),
            look_at: Vec3::new(-8.0, 0.4, 0.0),
            range: 8.0,
        },
        LabStation::Hold => StationProfile {
            player_pos: Vec3::new(-3.0, 1.6, 5.0),
            look_at: Vec3::new(-3.0, 0.6, 0.0),
            range: 8.0,
        },
        LabStation::Multi => StationProfile {
            player_pos: Vec3::new(2.0, 1.6, 5.0),
            look_at: Vec3::new(2.0, 0.7, 0.0),
            range: 8.0,
        },
        LabStation::Sequence => StationProfile {
            player_pos: Vec3::new(7.0, 1.6, 5.0),
            look_at: Vec3::new(7.0, 0.8, 0.0),
            range: 8.0,
        },
        LabStation::Gated => StationProfile {
            player_pos: Vec3::new(-5.0, 1.6, -5.0),
            look_at: Vec3::new(-5.0, 0.6, -10.0),
            range: 10.0,
        },
        LabStation::Vehicle => StationProfile {
            player_pos: Vec3::new(5.0, 1.6, -5.0),
            look_at: Vec3::new(5.0, 0.5, -10.0),
            range: 10.0,
        },
    }
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

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
    pub last_stage_transition: Option<String>,
    pub actor_powered: bool,
    pub actor_seated: bool,
    pub hold_to_toggle: bool,
}

#[derive(Component)]
struct LabOverlay;

// ---------------------------------------------------------------------------
// Station teleport input
// ---------------------------------------------------------------------------

use bevy_enhanced_input::prelude::*;

#[derive(InputAction)]
#[action_output(bool)]
struct Station1Action;
#[derive(InputAction)]
#[action_output(bool)]
struct Station2Action;
#[derive(InputAction)]
#[action_output(bool)]
struct Station3Action;
#[derive(InputAction)]
#[action_output(bool)]
struct Station4Action;
#[derive(InputAction)]
#[action_output(bool)]
struct Station5Action;
#[derive(InputAction)]
#[action_output(bool)]
struct Station6Action;
#[derive(InputAction)]
#[action_output(bool)]
struct TogglePowerAction;

#[derive(Component)]
struct LabInputContext;

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
const DEFAULT_BRP_PORT: u16 = 15_732;

fn main() -> AppExit {
    let mut app = common::base_app("interaction lab");

    app.insert_resource(InteractionDebugSettings {
        enabled: true,
        ..default()
    });
    app.init_resource::<LabDiagnostics>();
    app.register_type::<LabDiagnostics>();

    // Extra input context for station switching
    app.add_input_context::<LabInputContext>();

    #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
    {
        let port = std::env::var("BRP_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_BRP_PORT);
        app.add_plugins(BrpExtrasPlugin::with_port(port));
    }

    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::InteractionLabE2EPlugin);

    // Station teleport observers
    app.add_observer(on_station::<Station1Action, { LabStation::Instant as u8 }>);
    app.add_observer(on_station::<Station2Action, { LabStation::Hold as u8 }>);
    app.add_observer(on_station::<Station3Action, { LabStation::Multi as u8 }>);
    app.add_observer(on_station::<Station4Action, { LabStation::Sequence as u8 }>);
    app.add_observer(on_station::<Station5Action, { LabStation::Gated as u8 }>);
    app.add_observer(on_station::<Station6Action, { LabStation::Vehicle as u8 }>);
    app.add_observer(on_toggle_power);

    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            update_diagnostics,
            record_completed,
            record_canceled,
            record_failed,
            record_stage_advanced,
            handle_generator,
            handle_vehicle,
            update_lab_overlay,
        )
            .chain()
            .after(saddle_interaction::InteractionSystems::Feedback),
    );

    app.run()
}

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    common::spawn_environment(&mut commands, &mut meshes, &mut materials);

    // Player — starts at Station 1 (Instant)
    let player = common::spawn_player(&mut commands, Vec3::new(-8.0, 1.6, 5.0));
    commands.entity(player).insert((
        LabInputContext,
        actions!(LabInputContext[
            (Action::<Station1Action>::new(), bindings![KeyCode::Digit1]),
            (Action::<Station2Action>::new(), bindings![KeyCode::Digit2]),
            (Action::<Station3Action>::new(), bindings![KeyCode::Digit3]),
            (Action::<Station4Action>::new(), bindings![KeyCode::Digit4]),
            (Action::<Station5Action>::new(), bindings![KeyCode::Digit5]),
            (Action::<Station6Action>::new(), bindings![KeyCode::Digit6]),
            (Action::<TogglePowerAction>::new(), bindings![KeyCode::KeyP]),
        ]),
    ));

    // --- Station 1: Instant (Chest) ---
    let slots = vec![InteractionSlot::instant("open", "Open")];
    let chest = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Chest", common::PropShape::Cube(Vec3::new(1.0, 0.8, 0.7)),
        Vec3::new(-8.0, 0.4, 0.0), Color::srgb(0.55, 0.35, 0.15),
    );
    commands.entity(chest).insert((
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // --- Station 2: Hold (Valve) ---
    let slots = vec![InteractionSlot {
        behavior: InteractionBehavior::Single(InteractionExecution::Hold {
            duration_seconds: 0.75,
        }),
        ..InteractionSlot::instant("stabilize", "Stabilize")
    }];
    let valve = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Valve", common::PropShape::Cylinder { radius: 0.4, height: 1.2 },
        Vec3::new(-3.0, 0.6, 0.0), Color::srgb(0.7, 0.2, 0.15),
    );
    commands.entity(valve).insert((
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // --- Station 3: Multi-slot (Terminal) ---
    let slots = vec![
        InteractionSlot { priority: 1.1, ..InteractionSlot::instant("hack", "Hack") },
        InteractionSlot { priority: 0.5, ..InteractionSlot::instant("read", "Read") },
    ];
    let terminal = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Terminal", common::PropShape::Cube(Vec3::new(0.8, 1.4, 0.4)),
        Vec3::new(2.0, 0.7, 0.0), Color::srgb(0.15, 0.35, 0.65),
    );
    commands.entity(terminal).insert((
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // --- Station 4: Sequence (Lever) ---
    let slots = vec![InteractionSlot {
        behavior: InteractionBehavior::Sequence {
            stages: vec![
                InteractionStage { id: "prime".into(), execution: InteractionExecution::Instant, prompt: Some(InteractionPrompt { action_label_key: "Prime".into(), ..default() }) },
                InteractionStage { id: "pull".into(), execution: InteractionExecution::Instant, prompt: Some(InteractionPrompt { action_label_key: "Pull".into(), ..default() }) },
                InteractionStage { id: "reset".into(), execution: InteractionExecution::Instant, prompt: Some(InteractionPrompt { action_label_key: "Reset".into(), ..default() }) },
            ],
            advance_mode: SequenceAdvanceMode::Loop,
        },
        ..InteractionSlot::instant("lever", "Prime")
    }];
    let lever = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Lever", common::PropShape::Cylinder { radius: 0.15, height: 1.6 },
        Vec3::new(7.0, 0.8, 0.0), Color::srgb(0.5, 0.5, 0.55),
    );
    commands.entity(lever).insert((
        DemoBaseTargetSlots(slots.clone()),
        Interactable::default(),
        InteractionTarget { slots },
    ));

    // --- Station 5: Gated (Generator + Door) ---
    let gen_slots = vec![InteractionSlot::instant("activate", "Activate Generator")];
    let generator = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Generator", common::PropShape::Sphere(0.6),
        Vec3::new(-7.0, 0.6, -10.0), Color::srgb(0.8, 0.45, 0.1),
    );
    commands.entity(generator).insert((
        DemoBaseTargetSlots(gen_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: gen_slots },
    ));

    let door_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            required_actor_tags: vec![InteractionTag::from("powered")],
            ..default()
        },
        ..InteractionSlot::instant("unlock", "Unlock Door")
    }];
    let door = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Sealed Door", common::PropShape::Cube(Vec3::new(2.0, 2.5, 0.3)),
        Vec3::new(-3.0, 1.25, -10.0), Color::srgb(0.4, 0.12, 0.12),
    );
    commands.entity(door).insert((
        DemoBaseTargetSlots(door_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: door_slots },
    ));

    // --- Station 6: Vehicle (Cockpit + Exit) ---
    // Vehicle body
    commands.spawn((
        Name::new("Rover Body"),
        Mesh3d(meshes.add(Cuboid::new(2.5, 1.5, 5.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.4, 0.55),
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_translation(Vec3::new(5.0, 0.75, -10.0)),
    ));

    let enter_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            blocked_actor_tags: vec![InteractionTag::from("seated")],
            ..default()
        },
        reservation: InteractionReservationPolicy::Exclusive,
        ..InteractionSlot::instant("enter_vehicle", "Enter Rover")
    }];
    let cockpit = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Rover Cockpit", common::PropShape::Cube(Vec3::splat(0.5)),
        Vec3::new(5.0, 0.5, -8.0), Color::srgb(0.2, 0.6, 0.3),
    );
    commands.entity(cockpit).insert((
        DemoBaseTargetSlots(enter_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: enter_slots },
    ));

    let exit_slots = vec![InteractionSlot {
        availability: InteractionAvailabilityConfig {
            required_actor_tags: vec![InteractionTag::from("seated")],
            ..default()
        },
        reservation: InteractionReservationPolicy::Exclusive,
        ..InteractionSlot::instant("exit_vehicle", "Exit Rover")
    }];
    let hatch = common::spawn_prop(
        &mut commands, &mut meshes, &mut materials,
        "Exit Hatch", common::PropShape::Cube(Vec3::splat(0.5)),
        Vec3::new(5.0, 0.5, -12.0), Color::srgb(0.7, 0.3, 0.2),
    );
    commands.entity(hatch).insert((
        DemoBaseTargetSlots(exit_slots.clone()),
        Interactable::default(),
        InteractionTarget { slots: exit_slots },
    ));

    // Lab overlay (detailed diagnostics)
    commands.spawn((
        Name::new("Lab Overlay"),
        LabOverlay,
        Text::new(String::new()),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(16.0),
            max_width: Val::Px(420.0),
            ..default()
        },
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
    ));
}

// ---------------------------------------------------------------------------
// Station teleport
// ---------------------------------------------------------------------------

fn on_station<A: InputAction<Output = bool>, const S: u8>(
    _trigger: On<Start<A>>,
    mut players: Query<(&mut Transform, &mut DemoPlayerController, &mut Interactor), With<DemoPlayer>>,
) {
    let station = match S {
        0 => LabStation::Instant,
        1 => LabStation::Hold,
        2 => LabStation::Multi,
        3 => LabStation::Sequence,
        4 => LabStation::Gated,
        5 => LabStation::Vehicle,
        _ => return,
    };
    let profile = station_profile(station);
    let Ok((mut transform, mut ctrl, mut interactor)) = players.single_mut() else {
        return;
    };
    transform.translation = profile.player_pos;
    let dir = (profile.look_at - profile.player_pos).normalize_or_zero();
    ctrl.yaw = dir.x.atan2(dir.z);
    ctrl.pitch = 0.0;
    interactor.max_distance = Some(profile.range);
    interactor.proximity_radius = Some(profile.range);
}

fn on_toggle_power(
    _trigger: On<Start<TogglePowerAction>>,
    mut commands: Commands,
    players: Query<(Entity, &InteractionTags), With<DemoPlayer>>,
) {
    let Ok((entity, tags)) = players.single() else { return; };
    let powered = InteractionTag::from("powered");
    let mut new_tags = tags.clone();
    if new_tags.contains(&powered) {
        new_tags.tags.retain(|t| t != &powered);
    } else {
        new_tags.tags.push(powered);
    }
    commands.entity(entity).insert(new_tags);
}

// ---------------------------------------------------------------------------
// Diagnostics tracking
// ---------------------------------------------------------------------------

fn update_diagnostics(
    mut diag: ResMut<LabDiagnostics>,
    config: Res<InteractionConfig>,
    interactors: Query<
        (&FocusedInteraction, &InteractionPromptState, Option<&ActiveInteraction>, &InteractionTags),
        With<DemoPlayer>,
    >,
    names: Query<&Name>,
) {
    if let Some((focus, prompt_state, active, tags)) = interactors.iter().next() {
        diag.focused_target_name = focus.target.and_then(|e| names.get(e).ok()).map(|n| n.to_string());
        diag.focused_slot = focus.slot_id.as_ref().map(|s| s.0.clone());
        if let Some(offer) = &prompt_state.offer {
            diag.prompt_label = Some(offer.prompt.action_label_key.clone());
            diag.availability = offer.availability.as_ref().map(|r| format!("{r:?}"));
        } else {
            diag.prompt_label = None;
            diag.availability = None;
        }
        if let Some(a) = active {
            diag.active_slot = Some(a.slot_id.0.clone());
            diag.active_progress = a.progress;
        } else {
            diag.active_slot = None;
            diag.active_progress = 0.0;
        }
        diag.actor_powered = tags.contains(&InteractionTag::from("powered"));
        diag.actor_seated = tags.contains(&InteractionTag::from("seated"));
    }
    diag.hold_to_toggle = config.hold_to_toggle;
}

fn record_completed(mut diag: ResMut<LabDiagnostics>, mut reader: MessageReader<InteractionCompleted>) {
    for event in reader.read() {
        diag.completed_count += 1;
        diag.last_completed_slot = Some(event.slot_id.0.clone());
    }
}

fn record_canceled(mut diag: ResMut<LabDiagnostics>, mut reader: MessageReader<InteractionCanceled>) {
    for event in reader.read() {
        diag.canceled_count += 1;
        diag.last_canceled_reason = Some(format!("{:?}", event.reason));
    }
}

fn record_failed(mut diag: ResMut<LabDiagnostics>, mut reader: MessageReader<InteractionFailed>) {
    for event in reader.read() {
        diag.failed_count += 1;
        diag.last_failed_reason = Some(format!("{:?}", event.reason));
    }
}

fn record_stage_advanced(mut diag: ResMut<LabDiagnostics>, mut reader: MessageReader<InteractionStageAdvanced>) {
    for event in reader.read() {
        diag.stage_advanced_count += 1;
        let prev = event.previous_stage_id.as_ref().map(|s| s.0.as_str()).unwrap_or("start");
        let next = event.next_stage_id.as_ref().map(|s| s.0.as_str()).unwrap_or("end");
        diag.last_stage_transition = Some(format!("{prev} → {next}"));
    }
}

// ---------------------------------------------------------------------------
// Gameplay handlers
// ---------------------------------------------------------------------------

fn handle_generator(
    mut commands: Commands,
    mut reader: MessageReader<InteractionCompleted>,
    players: Query<&InteractionTags, With<DemoPlayer>>,
) {
    for event in reader.read() {
        if event.slot_id.0 != "activate" { continue; }
        let Ok(tags) = players.get(event.interactor) else { continue; };
        let mut new_tags = tags.clone();
        let powered = InteractionTag::from("powered");
        if !new_tags.contains(&powered) {
            new_tags.tags.push(powered);
        }
        commands.entity(event.interactor).insert(new_tags);
    }
}

fn handle_vehicle(
    mut commands: Commands,
    mut reader: MessageReader<InteractionCompleted>,
    mut players: Query<(&mut Transform, &mut DemoPlayerController, &InteractionTags), With<DemoPlayer>>,
) {
    for event in reader.read() {
        let Ok((mut transform, mut ctrl, tags)) = players.get_mut(event.interactor) else { continue; };
        let seated = InteractionTag::from("seated");
        match event.slot_id.0.as_str() {
            "enter_vehicle" => {
                let mut new_tags = tags.clone();
                new_tags.tags.push(seated);
                commands.entity(event.interactor).insert(new_tags);
                transform.translation = Vec3::new(5.0, 1.6, -10.0);
                ctrl.yaw = std::f32::consts::PI;
                ctrl.pitch = 0.0;
            }
            "exit_vehicle" => {
                let mut new_tags = tags.clone();
                new_tags.tags.retain(|t| t != &seated);
                commands.entity(event.interactor).insert(new_tags);
                transform.translation = station_profile(LabStation::Vehicle).player_pos;
                ctrl.yaw = 0.0;
                ctrl.pitch = 0.0;
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------

fn update_lab_overlay(
    diag: Res<LabDiagnostics>,
    mut overlay: Query<&mut Text, With<LabOverlay>>,
) {
    let Ok(mut text) = overlay.single_mut() else { return; };

    let focus = diag.focused_target_name.as_deref().unwrap_or("none");
    let prompt = diag.prompt_label.as_deref().unwrap_or("none");
    let avail = diag.availability.as_deref().unwrap_or("available");
    let active = diag.active_slot.as_deref().unwrap_or("none");
    let last_comp = diag.last_completed_slot.as_deref().unwrap_or("-");
    let last_cancel = diag.last_canceled_reason.as_deref().unwrap_or("-");
    let last_stage = diag.last_stage_transition.as_deref().unwrap_or("-");

    let tags = [
        if diag.actor_powered { "powered" } else { "" },
        if diag.actor_seated { "seated" } else { "" },
    ]
    .iter()
    .filter(|s| !s.is_empty())
    .copied()
    .collect::<Vec<_>>()
    .join(", ");
    let tags = if tags.is_empty() { "none".to_owned() } else { tags };

    **text = format!(
        "INTERACTION LAB\n\
         1-6: teleport  P: toggle power  WASD: move  E: interact\n\
         Tab/Q: cycle  Esc: cancel\n\n\
         focus: {focus}\n\
         prompt: {prompt}  [{avail}]\n\
         active: {active}  progress: {:.0}%\n\
         tags: {tags}\n\n\
         completed: {}  canceled: {}  failed: {}\n\
         stages: {}\n\
         last completed: {last_comp}\n\
         last canceled: {last_cancel}\n\
         last stage: {last_stage}",
        diag.active_progress * 100.0,
        diag.completed_count,
        diag.canceled_count,
        diag.failed_count,
        diag.stage_advanced_count,
    );
}

// ---------------------------------------------------------------------------
// Public helpers for E2E scenarios
// ---------------------------------------------------------------------------

pub fn go_to_station(world: &mut World, station: LabStation) {
    let profile = station_profile(station);
    let mut players = world.query_filtered::<(Entity, &mut Transform, &mut DemoPlayerController, &mut Interactor), With<DemoPlayer>>();
    let Ok((_entity, mut transform, mut ctrl, mut interactor)) = players.single_mut(world) else { return; };
    transform.translation = profile.player_pos;
    let dir = (profile.look_at - profile.player_pos).normalize_or_zero();
    ctrl.yaw = dir.x.atan2(dir.z);
    ctrl.pitch = 0.0;
    interactor.max_distance = Some(profile.range);
    interactor.proximity_radius = Some(profile.range);
}

pub fn send_intent(world: &mut World, kind: InteractionIntentKind) {
    let mut players = world.query_filtered::<Entity, With<DemoPlayer>>();
    let Ok(entity) = players.single(world) else { return; };
    world.write_message(InteractionIntent { interactor: entity, kind });
}

pub fn set_accessibility_toggle(world: &mut World, enabled: bool) {
    world.resource_mut::<InteractionConfig>().hold_to_toggle = enabled;
}

pub fn set_actor_powered(world: &mut World, enabled: bool) {
    let mut players = world.query_filtered::<(Entity, &InteractionTags), With<DemoPlayer>>();
    let Ok((entity, tags)) = players.single(world) else { return; };
    let powered = InteractionTag::from("powered");
    let mut new_tags = tags.clone();
    new_tags.tags.retain(|t| t != &powered);
    if enabled {
        new_tags.tags.push(powered);
    }
    world.entity_mut(entity).insert(new_tags);
}
