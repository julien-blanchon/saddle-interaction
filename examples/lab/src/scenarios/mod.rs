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
        "interaction_instant",
        "interaction_hold_complete",
        "interaction_hold_cancel",
        "interaction_multi_slot",
        "interaction_sequence",
        "interaction_gated",
        "interaction_vehicle",
        "interaction_accessibility",
    ]
}

pub fn scenario_by_name(name: &str) -> Option<Scenario> {
    match name {
        "smoke_launch" => Some(smoke_launch()),
        "interaction_instant" => Some(interaction_instant()),
        "interaction_hold_complete" => Some(interaction_hold_complete()),
        "interaction_hold_cancel" => Some(interaction_hold_cancel()),
        "interaction_multi_slot" => Some(interaction_multi_slot()),
        "interaction_sequence" => Some(interaction_sequence()),
        "interaction_gated" => Some(interaction_gated()),
        "interaction_vehicle" => Some(interaction_vehicle()),
        "interaction_accessibility" => Some(interaction_accessibility()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn station(station: LabStation) -> Action {
    Action::Custom(Box::new(move |world| go_to_station(world, station)))
}

fn power(enabled: bool) -> Action {
    Action::Custom(Box::new(move |world| set_actor_powered(world, enabled)))
}

fn accessibility(enabled: bool) -> Action {
    Action::Custom(Box::new(move |world| set_accessibility_toggle(world, enabled)))
}

fn intent(kind: InteractionIntentKind) -> Action {
    Action::Custom(Box::new(move |world| send_intent(world, kind.clone())))
}

fn reset() -> Vec<Action> {
    vec![accessibility(false), power(false)]
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

fn smoke_launch() -> Scenario {
    Scenario::builder("smoke_launch")
        .description("Boot the lab, settle, screenshot the instant station.")
        .then_many(reset())
        .then(station(LabStation::Instant))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("instant station has a prompt", |world| {
            world.resource::<LabDiagnostics>().prompt_label.is_some()
        }))
        .then(Action::Screenshot("smoke".into()))
        .then(Action::Log("smoke_launch: instant station focused".into()))
        .then(assertions::log_summary("smoke_launch"))
        .build()
}

fn interaction_instant() -> Scenario {
    Scenario::builder("interaction_instant")
        .description("Focus the chest, press E, verify instant completion.")
        .then_many(reset())
        .then(station(LabStation::Instant))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("chest is focused", |world| {
            world.resource::<LabDiagnostics>().focused_target_name.as_deref() == Some("Chest")
        }))
        .then(Action::Screenshot("instant_focused".into()))
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(3))
        .then(assertions::custom("chest interaction completed", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.last_completed_slot.as_deref() == Some("open") && d.completed_count == 1
        }))
        .then(Action::Screenshot("instant_completed".into()))
        .then(Action::Log("interaction_instant: completed open slot".into()))
        .then(assertions::log_summary("interaction_instant"))
        .build()
}

fn interaction_hold_complete() -> Scenario {
    Scenario::builder("interaction_hold_complete")
        .description("Start a hold interaction, verify mid-progress, then verify completion.")
        .then_many(reset())
        .then(station(LabStation::Hold))
        .then(Action::WaitFrames(10))
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(18))
        .then(assertions::custom("hold in progress", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.active_slot.as_deref() == Some("stabilize")
                && d.active_progress > 0.2
                && d.active_progress < 1.0
        }))
        .then(Action::Screenshot("hold_charging".into()))
        .then(Action::Log(format!("hold_complete: mid-progress verified")))
        .then(Action::WaitFrames(40))
        .then(assertions::custom("hold completed", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.last_completed_slot.as_deref() == Some("stabilize")
                && d.completed_count == 1
                && d.active_slot.is_none()
        }))
        .then(Action::Screenshot("hold_complete".into()))
        .then(Action::Log("hold_complete: stabilize completed".into()))
        .then(assertions::log_summary("interaction_hold_complete"))
        .build()
}

fn interaction_hold_cancel() -> Scenario {
    Scenario::builder("interaction_hold_cancel")
        .description("Begin a hold, release early, verify cancellation.")
        .then_many(reset())
        .then(station(LabStation::Hold))
        .then(Action::WaitFrames(10))
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(15))
        .then(assertions::custom("hold mid-progress", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.active_slot.as_deref() == Some("stabilize") && d.active_progress > 0.15
        }))
        .then(Action::Screenshot("hold_cancel_charging".into()))
        .then(intent(InteractionIntentKind::Release))
        .then(Action::WaitFrames(4))
        .then(assertions::custom("hold canceled", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.canceled_count == 1
                && d.last_canceled_reason.as_deref() == Some("InputReleased")
                && d.active_slot.is_none()
        }))
        .then(Action::Screenshot("hold_cancelled".into()))
        .then(Action::Log("hold_cancel: canceled after early release".into()))
        .then(assertions::log_summary("interaction_hold_cancel"))
        .build()
}

fn interaction_multi_slot() -> Scenario {
    Scenario::builder("interaction_multi_slot")
        .description("Verify default slot is Hack, cycle to Read, verify prompt change.")
        .then_many(reset())
        .then(station(LabStation::Multi))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("terminal starts on Hack", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.focused_target_name.as_deref() == Some("Terminal")
                && d.prompt_label.as_deref() == Some("Hack")
        }))
        .then(Action::Screenshot("multi_default".into()))
        .then(intent(InteractionIntentKind::CycleNext))
        .then(Action::WaitFrames(4))
        .then(assertions::custom("cycled to Read", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.focused_target_name.as_deref() == Some("Terminal")
                && d.prompt_label.as_deref() == Some("Read")
        }))
        .then(Action::Screenshot("multi_cycled".into()))
        .then(Action::Log("multi_slot: cycled from Hack to Read".into()))
        .then(assertions::log_summary("interaction_multi_slot"))
        .build()
}

fn interaction_sequence() -> Scenario {
    Scenario::builder("interaction_sequence")
        .description("Advance through Prime → Pull → Reset stages of the lever sequence.")
        .then_many(reset())
        .then(station(LabStation::Sequence))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("lever starts on Prime", |world| {
            world.resource::<LabDiagnostics>().prompt_label.as_deref() == Some("Prime")
        }))
        .then(Action::Screenshot("sequence_prime".into()))
        // Stage 1: Prime → Pull
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(4))
        .then(assertions::custom("stage advanced to Pull", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.stage_advanced_count >= 1
                && d.prompt_label.as_deref() == Some("Pull")
        }))
        .then(Action::Screenshot("sequence_pull".into()))
        // Stage 2: Pull → Reset
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(4))
        .then(assertions::custom("stage advanced to Reset", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.stage_advanced_count >= 2
                && d.prompt_label.as_deref() == Some("Reset")
        }))
        .then(Action::Screenshot("sequence_reset".into()))
        // Stage 3: Reset → loops back to Prime
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(4))
        .then(assertions::custom("sequence loops back to Prime", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.stage_advanced_count >= 3
                && d.prompt_label.as_deref() == Some("Prime")
        }))
        .then(Action::Screenshot("sequence_looped".into()))
        .then(Action::Log("sequence: completed full Prime→Pull→Reset→Prime cycle".into()))
        .then(assertions::log_summary("interaction_sequence"))
        .build()
}

fn interaction_gated() -> Scenario {
    Scenario::builder("interaction_gated")
        .description("Verify door is blocked, activate generator to gain powered tag, then unlock door.")
        .then_many(reset())
        .then(station(LabStation::Gated))
        .then(Action::WaitFrames(10))
        // Door should be blocked (no powered tag)
        .then(assertions::custom("door shows availability block", |world| {
            world.resource::<LabDiagnostics>().availability.is_some()
        }))
        .then(Action::Screenshot("gated_blocked".into()))
        // Grant powered tag and re-check
        .then(power(true))
        .then(Action::WaitFrames(6))
        .then(assertions::custom("door becomes available with powered tag", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.actor_powered && d.availability.is_none()
        }))
        .then(Action::Screenshot("gated_available".into()))
        // Unlock the door
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(3))
        .then(assertions::custom("door unlocked", |world| {
            world.resource::<LabDiagnostics>().last_completed_slot.as_deref() == Some("unlock")
        }))
        .then(Action::Screenshot("gated_unlocked".into()))
        .then(Action::Log("gated: unlocked door after gaining powered tag".into()))
        .then(assertions::log_summary("interaction_gated"))
        .build()
}

fn interaction_vehicle() -> Scenario {
    Scenario::builder("interaction_vehicle")
        .description("Enter rover, verify seated state, exit, verify unseated.")
        .then_many(reset())
        .then(station(LabStation::Vehicle))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("cockpit focused, not seated", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.focused_target_name.as_deref() == Some("Rover Cockpit")
                && d.prompt_label.as_deref() == Some("Enter Rover")
                && !d.actor_seated
        }))
        .then(Action::Screenshot("vehicle_ready".into()))
        // Enter
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(5))
        .then(assertions::custom("entered, seated, exit hatch focused", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.last_completed_slot.as_deref() == Some("enter_vehicle")
                && d.actor_seated
                && d.focused_target_name.as_deref() == Some("Exit Hatch")
                && d.prompt_label.as_deref() == Some("Exit Rover")
        }))
        .then(Action::Screenshot("vehicle_seated".into()))
        // Exit
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(5))
        .then(assertions::custom("exited, unseated, entry prompt restored", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.last_completed_slot.as_deref() == Some("exit_vehicle")
                && !d.actor_seated
                && d.prompt_label.as_deref() == Some("Enter Rover")
                && d.completed_count == 2
        }))
        .then(Action::Screenshot("vehicle_exit".into()))
        .then(Action::Log("vehicle: completed enter/exit cycle".into()))
        .then(assertions::log_summary("interaction_vehicle"))
        .build()
}

fn interaction_accessibility() -> Scenario {
    Scenario::builder("interaction_accessibility")
        .description("Enable hold-to-toggle, verify hold slot completes instantly on single press.")
        .then(accessibility(true))
        .then(power(false))
        .then(station(LabStation::Hold))
        .then(Action::WaitFrames(10))
        .then(assertions::custom("hold-to-toggle enabled", |world| {
            world.resource::<LabDiagnostics>().hold_to_toggle
        }))
        .then(Action::Screenshot("accessibility_ready".into()))
        .then(intent(InteractionIntentKind::Press))
        .then(Action::WaitFrames(3))
        .then(assertions::custom("hold completes instantly in toggle mode", |world| {
            let d = world.resource::<LabDiagnostics>();
            d.last_completed_slot.as_deref() == Some("stabilize")
                && d.completed_count == 1
                && d.active_slot.is_none()
        }))
        .then(Action::Screenshot("accessibility_completed".into()))
        .then(Action::Log("accessibility: hold completed instantly with toggle mode".into()))
        .then(assertions::log_summary("interaction_accessibility"))
        .build()
}
