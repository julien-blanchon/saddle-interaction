# saddle-interaction-lab

Crate-local lab for the `saddle-interaction` shared crate.

## Run

```bash
cargo run -p saddle-interaction-lab
```

The lab renders four interaction stations:

- focus arbitration between a closer low-priority prop and a farther high-priority relay
- a hold-to-complete console
- a multi-action panel
- a gated door prompt that stays unavailable until the actor gains a tag

The overlay shows the current prompt, active progress, last lifecycle result, and whether accessibility hold-to-toggle is enabled.

Keyboard controls:

- `1` / `2` / `3` / `4`: jump the interactor between the showcase stations
- `E`: send confirm
- `Esc`: cancel the active interaction
- `Tab` / `Q`: cycle forward or backward through multi-action slots
- `P`: toggle the actor's `powered` tag for the gated door
- `T`: toggle accessibility `hold_to_toggle`

## BRP

The lab enables BRP in the default `dev` feature set.

```bash
BRP_EXTRAS_PORT=15732 cargo run -p saddle-interaction-lab
BRP_PORT=15732 uv run --active --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
```

Useful runtime inspection targets:

- `saddle_interaction::components::FocusedInteraction`
- `saddle_interaction::components::InteractionPromptState`
- `saddle_interaction::components::ActiveInteraction`
- `saddle_interaction::config::InteractionConfig`
- `saddle_interaction_lab::LabDiagnostics`

## E2E

```bash
cargo run -p saddle-interaction-lab --features e2e -- interaction_smoke
```

Available scenarios:

- `smoke_launch`
- `interaction_smoke`
- `interaction_focus_priority`
- `interaction_hold_complete`
- `interaction_hold_cancel`
- `interaction_multi_action_prompt`
- `interaction_accessibility_toggle_mode`
