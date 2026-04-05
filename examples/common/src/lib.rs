use bevy::prelude::*;
use saddle_interaction::{
    Interactable, InteractionBehavior, InteractionConfig, InteractionExecution, InteractionSlot,
    InteractionTarget, Interactor,
};
use saddle_pane::prelude::*;

/// Marker for the primary interactor entity in demos.
#[derive(Component)]
pub struct DemoInteractor;

/// Stores the original slot configuration so the pane can reset tuned values.
#[derive(Component, Clone)]
pub struct DemoBaseTargetSlots(pub Vec<InteractionSlot>);

// ---------------------------------------------------------------------------
// Pane plugins bundle (re-used by dialogue_terminal, lab, and individual demos)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Shared demo pane (live-tuning of interaction parameters)
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
struct InteractionDemoPaneInstalled;

#[derive(Resource, Clone, Default, Pane)]
#[pane(title = "Interaction Tuning")]
pub struct InteractionDemoPane {
    #[pane(slider, min = 50.0, max = 800.0, step = 10.0)]
    pub actor_range: f32,
    #[pane(slider, min = 0.5, max = 2.5, step = 0.05)]
    pub detection_radius_scale: f32,
    #[pane(slider, min = 0.25, max = 2.5, step = 0.05)]
    pub hold_time_scale: f32,
    pub hold_to_toggle: bool,
    pub auto_interact_on_focus: bool,
}

impl InteractionDemoPane {
    pub fn new(hold_to_toggle: bool) -> Self {
        Self {
            actor_range: 500.0,
            detection_radius_scale: 1.0,
            hold_time_scale: 1.0,
            hold_to_toggle,
            auto_interact_on_focus: false,
        }
    }
}

/// Register the shared interaction-tuning pane. Safe to call multiple times.
pub fn install_demo_pane(app: &mut App, hold_to_toggle: bool) {
    if app
        .world()
        .contains_resource::<InteractionDemoPaneInstalled>()
    {
        return;
    }

    app.insert_resource(InteractionDemoPaneInstalled);
    if !app.world().contains_resource::<InteractionDemoPane>() {
        app.insert_resource(InteractionDemoPane::new(hold_to_toggle));
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
    mut targets: Query<(
        &mut Interactable,
        &mut InteractionTarget,
        &DemoBaseTargetSlots,
    )>,
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
