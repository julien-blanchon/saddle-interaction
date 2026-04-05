# Saddle Interaction

Reusable world-interaction substrate for Bevy: candidate detection, arbitration, sticky focus, gated prompts, hold or toggle execution, chained stages, cooldowns, exclusive reservations, and lifecycle messages.

The crate stays generic. It decides which interaction is currently offered and when it starts, progresses, completes, or cancels. Consumer crates own the actual gameplay consequence such as opening a door, starting dialogue, consuming inventory, or playing bespoke VFX.

## Quick Start

```toml
[dependencies]
saddle-interaction = { git = "https://github.com/julien-blanchon/saddle-interaction" }
```

```rust
use bevy::prelude::*;
use saddle_interaction::{
    Interactable, InteractionIntent, InteractionIntentKind, InteractionPlugin, InteractionSlot,
    InteractionTarget, Interactor,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InteractionPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, trigger_interaction)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Interactor"),
        Interactor::default(),
        GlobalTransform::IDENTITY,
    ));

    commands.spawn((
        Name::new("Terminal"),
        Interactable::default(),
        InteractionTarget {
            slots: vec![InteractionSlot::instant("inspect", "Inspect")],
        },
        GlobalTransform::from_xyz(1.5, 0.0, 0.0),
    ));
}

fn trigger_interaction(
    interactor: Single<Entity, With<Interactor>>,
    mut intents: MessageWriter<InteractionIntent>,
) {
    intents.write(InteractionIntent {
        interactor: *interactor,
        kind: InteractionIntentKind::Press,
    });
}
```

## Plugin Usage

`InteractionPlugin` preserves injectable schedules:

```rust
use bevy::prelude::*;
use saddle_interaction::InteractionPlugin;

app.add_plugins(InteractionPlugin::new(
    OnEnter(MyState::Gameplay),
    OnExit(MyState::Gameplay),
    Update,
));
```

For always-on examples, tests, and labs:

```rust
app.add_plugins(InteractionPlugin::always_on(Update));
```

## Public API

Plugin and ordering:

- `InteractionPlugin`
- `InteractionSystems::{Detect, Score, Focus, Gate, Execute, Feedback}`

Core actor and target data:

- `Interactor`, `InteractorAim`, `InteractorPointer`
- `Interactable`, `InteractionTarget`, `InteractionSlot`
- `InteractionPrompt`, `InteractionPromptState`
- `FocusedInteraction`, `InteractionCandidates`, `ActiveInteraction`
- `InteractionTags`, `InteractionChannel`, `InteractionOccluder`

Config and diagnostics:

- `InteractionConfig`
- `InteractionPredicateRegistry`
- `InteractionStats`
- `InteractionDebugSettings`

Lifecycle and intent messages:

- `InteractionIntent`
- `FocusChanged`
- `InteractionOffered`
- `InteractionStarted`
- `InteractionProgress`
- `InteractionCompleted`
- `InteractionCanceled`
- `InteractionFailed`
- `InteractionStageAdvanced`

## Prompt and UI Decoupling

The crate never renders prompt UI directly.

- `InteractionPromptState` stores the currently offered slot as data.
- `InteractionOffered` emits prompt refresh facts for HUD or diegetic widgets.
- `InteractionFocusedBy` marks focused targets so consumer crates can render highlights or outlines.
- `InteractionPrompt` keeps semantic fields such as `action_label_key`, `input_hint_key`, `icon_key`, and anchor metadata without assuming any concrete UI implementation.

That split keeps prompt presentation reusable across HUD text, world-space widgets, accessibility overlays, or audio cues.

## Input Integration

The runtime listens to `InteractionIntent` messages, not raw keyboard or mouse polling. This keeps the public surface stable across keyboard, gamepad, remapping layers, AI drivers, replay systems, and E2E harnesses.

The crate examples use `bevy_enhanced_input` to translate gameplay actions into `InteractionIntentKind::{Press, Release, Cancel, CycleNext, CyclePrevious, SelectSlot}`.

## Examples

| Example | Command | What it demonstrates |
| --- | --- | --- |
| `basic` | `cargo run -p saddle-interaction-example-basic` | One interactor, one instant target |
| `hold` | `cargo run -p saddle-interaction-example-hold` | Hold-to-complete progress and completion feedback |
| `multi_action` | `cargo run -p saddle-interaction-example-multi-action` | One target with multiple slots and cycling |
| `chained` | `cargo run -p saddle-interaction-example-chained` | Multi-stage slot progression |
| `gated` | `cargo run -p saddle-interaction-example-gated` | Tag-gated availability and unlock flow |
| `accessibility` | `cargo run -p saddle-interaction-example-accessibility` | `hold_to_toggle` accessibility transform |
| `prompt_ui` | `cargo run -p saddle-interaction-example-prompt-ui` | HUD prompt integration from prompt state only |
| `vehicle_bay` | `cargo run -p saddle-interaction-example-vehicle-bay` | Exclusive seat reservation plus enter / exit gating |
| `dialogue_terminal` | `cargo run -p saddle-interaction-example-dialogue-terminal` | Cross-crate comms terminal using interaction triggers, tweened UI panels, and animated dialogue text |
| `saddle-interaction-lab` | `cargo run -p saddle-interaction-lab` | Rich crate-local showcase with BRP and E2E scenarios |

## Crate-Local Lab

The crate ships a richer lab app under [`examples/lab/README.md`](examples/lab/README.md). It exposes:

- arbitration between competing targets
- hold progress and cancellation
- multi-slot prompt switching
- gated prompt availability
- BRP-friendly diagnostics via `LabDiagnostics`
- E2E scenarios for smoke, focus priority, hold complete, hold cancel, prompt switching, and accessibility toggle mode

## 2D Projects

With `Camera2d`, one world unit equals one screen pixel. The default `InteractionConfig` distances (3–4 units) are tuned for 3D scenes. For 2D, override per-actor or globally:

```rust
Interactor {
    max_distance: Some(500.0),
    proximity_radius: Some(500.0),
    ..default()
}
```

See `docs/configuration.md` → **2D (Camera2d) Tuning** for details.

## Consumer Responsibilities

The crate intentionally does not own:

- inventory, quest, dialogue, or save-game logic
- door or terminal behavior
- visual outline rendering
- localised input glyph generation

Consumer systems should read the crate’s public messages and components, then apply project-specific world effects.

## Documentation

- [`docs/architecture.md`](docs/architecture.md)
- [`docs/configuration.md`](docs/configuration.md)
