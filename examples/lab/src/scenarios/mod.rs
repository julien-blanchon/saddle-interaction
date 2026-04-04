use bevy::prelude::*;
use saddle_bevy_e2e::{action::Action, actions::assertions, scenario::Scenario};
use saddle_interaction::InteractionIntentKind;

use crate::{
    LabDiagnostics, LabStation, go_to_station, send_intent, set_accessibility_toggle,
    set_actor_powered,
};

pub fn list_scenarios() -> Vec<&'static str> {
    vec![
        "smoke_launch",
        "interaction_smoke",
        "interaction_focus_priority",
        "interaction_hold_complete",
        "interaction_hold_cancel",
        "interaction_multi_action_prompt",
        "interaction_accessibility_toggle_mode",
        "interaction_vehicle_bay",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke_launch()),
        "interaction_smoke" => Some(interaction_smoke()),
        "interaction_focus_priority" => Some(interaction_focus_priority()),
        "interaction_hold_complete" => Some(interaction_hold_complete()),
        "interaction_hold_cancel" => Some(interaction_hold_cancel()),
        "interaction_multi_action_prompt" => Some(interaction_multi_action_prompt()),
        "interaction_accessibility_toggle_mode" => Some(interaction_accessibility_toggle_mode()),
        "interaction_vehicle_bay" => Some(interaction_vehicle_bay()),
        _ => None,
    }
}

fn station(station: LabStation) -> Action {
    Action::Custom(Box::new(move |world| go_to_station(world, station)))
}

fn power(enabled: bool) -> Action {
    Action::Custom(Box::new(move |world| set_actor_powered(world, enabled)))
}

fn accessibility(enabled: bool) -> Action {
    Action::Custom(Box::new(move |world| {
        set_accessibility_toggle(world, enabled)
    }))
}

fn intent(kind: InteractionIntentKind) -> Action {
    Action::Custom(Box::new(move |world| send_intent(world, kind.clone())))
}

fn smoke_launch() -> Scenario {
    Scenario::builder("smoke_launch")
        .description("Boot the crate-local lab, settle the focus state, and capture the default arbitration station.")
        .then(accessibility(false))
        .then(power(false))
        .then(station(LabStation::Priority))
        .then(Action::WaitFrames(8))
        .then(assertions::custom("priority station initializes a prompt", |world| {
            world.resource::<LabDiagnostics>().prompt_label.is_some()
        }))
        .then(Action::Screenshot("smoke".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("smoke_launch"))
        .build()
}

fn interaction_smoke() -> Scenario {
    Scenario::builder("interaction_smoke")
        .description("Verify the lab can focus a valid target, then move to a gated target and confirm the unavailable prompt reason is surfaced.")
        .then(accessibility(false))
        .then(power(false))
        .then(station(LabStation::Priority))
        .then(Action::WaitFrames(8))
        .then(assertions::custom("priority relay is focused", |world| {
            world.resource::<LabDiagnostics>().focused_target_name.as_deref() == Some("Priority Relay")
        }))
        .then(Action::Screenshot("priority_ready".into()))
        .then(Action::WaitFrames(1))
        .then(station(LabStation::Gated))
        .then(Action::WaitFrames(8))
        .then(assertions::custom("gated prompt stays unavailable without the powered tag", |world| {
            world
                .resource::<LabDiagnostics>()
                .availability
                .as_deref()
                == Some("missing_actor_tag:powered")
        }))
        .then(Action::Screenshot("gated_unavailable".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("interaction_smoke"))
        .build()
}

fn interaction_focus_priority() -> Scenario {
    Scenario::builder("interaction_focus_priority")
        .description("Show that the farther high-priority relay beats the closer low-priority crate during arbitration.")
        .then(accessibility(false))
        .then(power(false))
        .then(station(LabStation::Priority))
        .then(Action::WaitFrames(8))
        .then(assertions::custom("priority relay wins arbitration", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.focused_target_name.as_deref() == Some("Priority Relay")
                && diagnostics.prompt_label.as_deref() == Some("Reroute")
        }))
        .then(Action::Screenshot("focus_priority".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("interaction_focus_priority"))
        .build()
}

fn interaction_hold_complete() -> Scenario {
    Scenario::builder("interaction_hold_complete")
        .description("Start a hold interaction, capture an in-progress frame, then verify the hold completes and clears active state.")
        .then(accessibility(false))
        .then(power(false))
        .then(station(LabStation::Hold))
        .then(Action::WaitFrames(8))
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(18))
        .then(assertions::custom("hold interaction reports in-progress state", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.active_slot.as_deref() == Some("stabilize")
                && diagnostics.active_progress > 0.2
                && diagnostics.active_progress < 1.0
        }))
        .then(Action::Screenshot("hold_charging".into()))
        .then(Action::WaitFrames(1))
        .then(Action::WaitFrames(34))
        .then(assertions::custom("hold interaction completes", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.last_completed_slot.as_deref() == Some("stabilize")
                && diagnostics.completed_count == 1
                && diagnostics.active_slot.is_none()
        }))
        .then(Action::Screenshot("hold_complete".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("interaction_hold_complete"))
        .build()
}

fn interaction_hold_cancel() -> Scenario {
    Scenario::builder("interaction_hold_cancel")
        .description("Begin a hold interaction, release early, and verify cancel feedback removes active progress.")
        .then(accessibility(false))
        .then(power(false))
        .then(station(LabStation::Hold))
        .then(Action::WaitFrames(8))
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(15))
        .then(assertions::custom("hold cancel scenario reaches mid-progress", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.active_slot.as_deref() == Some("stabilize") && diagnostics.active_progress > 0.15
        }))
        .then(Action::Screenshot("hold_cancel_charging".into()))
        .then(Action::WaitFrames(1))
        .then(intent(InteractionIntentKind::Release))
        .then(Action::WaitFrames(4))
        .then(assertions::custom("hold cancel emits input released", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.canceled_count == 1
                && diagnostics.last_canceled_reason.as_deref() == Some("input_released")
                && diagnostics.active_slot.is_none()
        }))
        .then(Action::Screenshot("hold_cancelled".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("interaction_hold_cancel"))
        .build()
}

fn interaction_multi_action_prompt() -> Scenario {
    Scenario::builder("interaction_multi_action_prompt")
        .description("Focus a target with multiple slots, capture the default highest-priority prompt, cycle, and verify the prompt swaps without changing the target.")
        .then(accessibility(false))
        .then(power(false))
        .then(station(LabStation::Multi))
        .then(Action::WaitFrames(8))
        .then(assertions::custom("multi-action target starts on the high-priority slot", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.focused_target_name.as_deref() == Some("Service Panel")
                && diagnostics.prompt_label.as_deref() == Some("Hack")
        }))
        .then(Action::Screenshot("multi_default".into()))
        .then(Action::WaitFrames(1))
        .then(intent(InteractionIntentKind::CycleNext))
        .then(Action::WaitFrames(4))
        .then(assertions::custom("cycle next selects the alternate slot", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.focused_target_name.as_deref() == Some("Service Panel")
                && diagnostics.prompt_label.as_deref() == Some("Read")
        }))
        .then(Action::Screenshot("multi_cycled".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("interaction_multi_action_prompt"))
        .build()
}

fn interaction_accessibility_toggle_mode() -> Scenario {
    Scenario::builder("interaction_accessibility_toggle_mode")
        .description("Enable hold-to-toggle accessibility mode and verify the same hold slot completes immediately after a single confirm press.")
        .then(accessibility(true))
        .then(power(false))
        .then(station(LabStation::Hold))
        .then(Action::WaitFrames(8))
        .then(assertions::custom("hold-to-toggle accessibility mode is enabled", |world| {
            world.resource::<LabDiagnostics>().hold_to_toggle
        }))
        .then(Action::Screenshot("accessibility_ready".into()))
        .then(Action::WaitFrames(1))
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(3))
        .then(assertions::custom("press completes the hold slot without an active timer", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.last_completed_slot.as_deref() == Some("stabilize")
                && diagnostics.completed_count == 1
                && diagnostics.active_slot.is_none()
        }))
        .then(Action::Screenshot("accessibility_toggle".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("interaction_accessibility_toggle_mode"))
        .build()
}

fn interaction_vehicle_bay() -> Scenario {
    Scenario::builder("interaction_vehicle_bay")
        .description("Enter the rover through the exclusive cockpit slot, verify the seated exit flow appears, then exit back to the staging pad.")
        .then(accessibility(false))
        .then(power(false))
        .then(station(LabStation::Vehicle))
        .then(Action::WaitFrames(8))
        .then(assertions::custom("vehicle bay starts focused on the cockpit entry slot", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.focused_target_name.as_deref() == Some("Rover Cockpit")
                && diagnostics.prompt_label.as_deref() == Some("Enter Rover")
                && !diagnostics.actor_seated
        }))
        .then(Action::Screenshot("vehicle_ready".into()))
        .then(Action::WaitFrames(1))
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(5))
        .then(assertions::custom("entering the rover grants the seated tag and surfaces the exit slot", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.last_completed_slot.as_deref() == Some("enter_vehicle")
                && diagnostics.actor_seated
                && diagnostics.focused_target_name.as_deref() == Some("Exit Hatch")
                && diagnostics.prompt_label.as_deref() == Some("Exit Rover")
        }))
        .then(Action::Screenshot("vehicle_seated".into()))
        .then(Action::WaitFrames(1))
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(5))
        .then(assertions::custom("exiting the rover clears the seated tag and restores the entry prompt", |world| {
            let diagnostics = world.resource::<LabDiagnostics>();
            diagnostics.last_completed_slot.as_deref() == Some("exit_vehicle")
                && !diagnostics.actor_seated
                && diagnostics.prompt_label.as_deref() == Some("Enter Rover")
                && diagnostics.completed_count == 2
        }))
        .then(Action::Screenshot("vehicle_exit".into()))
        .then(Action::WaitFrames(1))
        .then(assertions::log_summary("interaction_vehicle_bay"))
        .build()
}
