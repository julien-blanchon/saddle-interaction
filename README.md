# Saddle Interaction

Reusable world-interaction substrate for Bevy: candidate detection, arbitration, sticky focus, gated prompts, hold or toggle execution, chained stages, cooldowns, exclusive reservations, and lifecycle messages.

The crate stays generic. It decides which interaction is currently offered and when it starts, progresses, completes, or cancels. Consumer crates own the actual gameplay consequence such as opening a door, starting dialogue, consuming inventory, or playing bespoke VFX.

## When to Use

Use saddle-interaction when you need **discrete, prompt-driven interactions** between actors and world objects:

- Doors, levers, switches, buttons
- NPCs (talk, trade, recruit)
- Terminals, computers, control panels
- Vehicles (enter/exit)
- Chests, containers, pickups
- Crafting stations, workbenches
- Multi-step rituals (chained sequences)
- Context-sensitive actions that change based on player state (tags)

## When NOT to Use

| Scenario | Why not | Use instead |
|----------|---------|-------------|
| Continuous physics manipulation (grab, carry, throw) | Needs spring-damper hold, physics forces | `saddle-physics-object-interaction` |
| Pure UI interactions (menus, inventory slots) | No spatial detection needed | Bevy UI / `bevy_ui_widgets` |
| Automated world changes (timer-based, no player intent) | No actor involvement | Direct systems/commands |
| Combat hit detection | Frame-precise hitbox/hurtbox, not proximity-prompt | Avian3D sensors / custom |

## Comparison with `saddle-physics-object-interaction`

These two crates are **orthogonal, not competing**. A game typically uses both:

| Feature | `saddle-interaction` | `saddle-physics-object-interaction` |
|---------|---------------------|-------------------------------------|
| Detection | Proximity, picking, hybrid | Raycast + sphere-cast from camera |
| Execution | Instant, hold, toggle, mash, sequence | Grab, carry, inspect, throw |
| Physics | None (pure ECS) | Avian3D spring-damper rigid bodies |
| Prompts | Decoupled data-driven prompts | Consumer-rendered (data only) |
| Best for | Doors, terminals, switches, vehicles | Props, weapons, puzzle pieces |

See the `pickup_prompt` example for how the two crates compose: saddle-interaction handles the prompt and focus, then bridges to the physics crate for the actual grab.

## Quick Start

```toml
[dependencies]
saddle-interaction = { git = "https://github.com/julien-blanchon/saddle-interaction" }
```

```rust
use bevy::prelude::*;
use saddle_interaction::{
    Interactable, InteractionCompleted, InteractionIntent, InteractionIntentKind,
    InteractionPlugin, InteractionSlot, InteractionTarget, Interactor, InteractionTags,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InteractionPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, on_door_opened)
        .run();
}

fn setup(mut commands: Commands) {
    // Player with interaction capability
    commands.spawn((
        Name::new("Player"),
        Interactor::default(),
        InteractionTags::default(),
        Transform::from_xyz(0.0, 1.6, 5.0),
    ));

    // Door with a single "Open" action
    commands.spawn((
        Name::new("Door"),
        Interactable::default(),
        InteractionTarget {
            slots: vec![InteractionSlot::instant("open", "Open")],
        },
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
}

fn on_door_opened(mut reader: MessageReader<InteractionCompleted>) {
    for event in reader.read() {
        info!("Door opened by {:?}!", event.interactor);
    }
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

## Pipeline

```text
InteractionIntent
    |
    v
Detect --> Score --> Focus --> Gate --> Execute --> Feedback
(spatial)  (rank)   (hyster-  (slot   (start/    (markers,
 index,     candi-   esis,    avail-   tick/      messages,
 collect)   dates)   lock)    check)   cancel/    prompts)
                                       finish)
```

Six system sets, all chained: `InteractionSystems::{Detect, Score, Focus, Gate, Execute, Feedback}`.

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

## Common Patterns

### Crosshair-style aim-to-focus

In FPS games, you typically only want to show the interaction prompt for the object the player is **looking directly at** — not everything nearby. Achieve this by boosting `alignment_weight` and reducing `distance_weight`:

```rust
Interactor {
    // High alignment means the dot(aim, forward_to_target) dominates scoring.
    // Objects off-center score so low they never win focus.
    alignment_weight: 3.0,
    // Low distance means nearby-but-off-screen objects don't steal focus.
    distance_weight: 0.2,
    require_line_of_sight: true,
    ..default()
}
```

Pair this with `InteractorAim` synced to the camera forward each frame:

```rust
fn sync_aim(
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    mut players: Query<&mut InteractorAim, With<Player>>,
) {
    let Ok(cam) = cameras.single() else { return };
    for mut aim in &mut players {
        aim.direction = cam.forward().into();
    }
}
```

See the `aim_focus` example for a full working demo with a center-screen crosshair.

### Dynamic enable/disable from code

Toggle an object's interactability at runtime by setting `Interactable.enabled`:

```rust
fn toggle_interactable(mut query: Query<&mut Interactable, With<MyTarget>>) {
    for mut interactable in &mut query {
        interactable.enabled = !interactable.enabled;
    }
}
```

When `enabled` is `false`, the object is skipped entirely by the detection pipeline — no focus, no prompt, no interaction possible.

### Runtime slot reconfiguration

Swap a slot's behavior at runtime by mutating `InteractionTarget.slots`:

```rust
fn make_slot_hold(mut targets: Query<&mut InteractionTarget, With<MyTarget>>) {
    for mut target in &mut targets {
        if let Some(slot) = target.slots.first_mut() {
            slot.behavior = InteractionBehavior::Single(
                InteractionExecution::Hold { duration_seconds: 2.0 },
            );
            slot.prompt.action_label_key = "Charge".into();
        }
    }
}
```

This is useful for context-sensitive interactions that change based on game state — a lock that becomes "Pick Lock" when the player has lockpicks, or a terminal that switches from "Read" to "Hack" based on the player's skills.

## Examples

### Standalone (saddle-interaction only)

| Example | Command | What it demonstrates |
|---------|---------|---------------------|
| `basic` | `cargo run -p saddle-interaction-example-basic` | Walk to a chest, press E to open (instant interaction) |
| `hold` | `cargo run -p saddle-interaction-example-hold` | Walk to a valve, hold E to turn it (progress bar, cancellation) |
| `multi_slot` | `cargo run -p saddle-interaction-example-multi-slot` | Terminal with three actions, Tab/Q to cycle between slots |
| `sequence` | `cargo run -p saddle-interaction-example-sequence` | Lever with three stages (Prime, Pull, Reset), loops |
| `gated` | `cargo run -p saddle-interaction-example-gated` | Generator + door: tag-gated availability |
| `vehicle` | `cargo run -p saddle-interaction-example-vehicle` | Enter/exit vehicle with exclusive reservation and tags |
| `aim_focus` | `cargo run -p saddle-interaction-example-aim-focus` | Crosshair aim-to-focus, dynamic enable/disable, runtime slot swap |
| `lab` | `cargo run -p saddle-interaction-lab` | All features combined, BRP inspection, E2E scenarios |

### Cross-Crate Integration (Tier 1)

| Example | Crates | Command | What it shows |
|---------|--------|---------|---------------|
| `fps_interactive` | + `saddle-character-controller` | `cargo run -p saddle-interaction-example-fps-interactive` | Physics FPS movement + interaction prompts on door, switch, console |
| `pickup_prompt` | + `saddle-physics-object-interaction` | `cargo run -p saddle-interaction-example-pickup-prompt` | Prompt-before-grab pipeline: interaction focus -> physics hold |
| `vehicle_entry` | + `saddle-vehicle-ground-vehicle` | `cargo run -p saddle-interaction-example-vehicle-entry` | Enter/exit real vehicle with mode switching |

## Lab and E2E Scenarios

The lab (`examples/lab/`) combines all features in one 3D scene with six stations. It supports:

- **BRP inspection** via `LabDiagnostics` resource (`--features dev`)
- **E2E scenarios** for automated testing (`--features e2e`)

Available scenarios:

```bash
cargo run -p saddle-interaction-lab --features e2e -- smoke_launch
cargo run -p saddle-interaction-lab --features e2e -- interaction_instant
cargo run -p saddle-interaction-lab --features e2e -- interaction_hold_complete
cargo run -p saddle-interaction-lab --features e2e -- interaction_hold_cancel
cargo run -p saddle-interaction-lab --features e2e -- interaction_multi_slot
cargo run -p saddle-interaction-lab --features e2e -- interaction_sequence
cargo run -p saddle-interaction-lab --features e2e -- interaction_gated
cargo run -p saddle-interaction-lab --features e2e -- interaction_vehicle
cargo run -p saddle-interaction-lab --features e2e -- interaction_accessibility
```

## 2D Projects

With `Camera2d`, one world unit equals one screen pixel. The default `InteractionConfig` distances (3-4 units) are tuned for 3D scenes. For 2D, override per-actor or globally:

```rust
Interactor {
    max_distance: Some(500.0),
    proximity_radius: Some(500.0),
    ..default()
}
```

See `docs/configuration.md` for details.

## Consumer Responsibilities

The crate intentionally does not own:

- inventory, quest, dialogue, or save-game logic
- door or terminal behavior
- visual outline rendering
- localised input glyph generation

Consumer systems should read the crate's public messages and components, then apply project-specific world effects.

## Documentation

- [`docs/architecture.md`](docs/architecture.md) — pipeline design, arbitration, execution state machine
- [`docs/configuration.md`](docs/configuration.md) — all tunable parameters with defaults
