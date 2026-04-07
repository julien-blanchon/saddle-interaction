# saddle-interaction architecture

## Pipeline Diagram

```text
InteractionIntent
    |
    v
+----------+   +----------+   +----------+   +-------+   +----------+   +-----------+
|  Detect  |-->|  Score   |-->|  Focus   |-->|  Gate |-->| Execute  |-->| Feedback  |
|          |   |          |   |          |   |       |   |          |   |           |
| rebuild  |   | rank     |   | hyster-  |   | slot  |   | start/   |   | markers   |
| spatial  |   | candi-   |   | esis     |   | avail |   | tick/    |   | messages  |
| index    |   | dates    |   | focus    |   | check |   | cancel/  |   | prompts   |
| collect  |   |          |   | lock     |   |       |   | finish   |   |           |
+----------+   +----------+   +----------+   +-------+   +----------+   +-----------+
```

## Message Flow (typical successful interaction)

```text
Frame N:   Player walks near target → Detect collects candidate
Frame N+1: Score ranks it top → Focus locks on → Gate evaluates slot
           → InteractionOffered emitted → FocusChanged emitted
Frame N+2: Player presses E → InteractionIntent(Press) consumed
           → Execute starts → InteractionStarted emitted
Frame N+3..M: (Hold) Execute ticks → InteractionProgress emitted each frame
Frame M:   Progress reaches 1.0 → InteractionCompleted emitted
           → ActiveInteraction removed
```

## Candidate Lifecycle

The runtime follows one stable pipeline each update:

1. `Detect`
   The crate prepares interactor-side runtime components, consumes `InteractionIntent`, rebuilds the spatial index, and collects proximity or picking-backed candidates.
2. `Score`
   Candidates are ranked per interactor using distance, alignment, target priority, slot priority, and picking bias.
3. `Focus`
   The top candidate becomes the current focused target unless hysteresis keeps the previous focus.
4. `Gate`
   The focused target resolves one concrete slot, evaluates availability, and writes `InteractionPromptState`.
5. `Execute`
   The runtime starts, ticks, completes, or cancels `ActiveInteraction`.
6. `Feedback`
   Focus markers and lifecycle messages are emitted for consumers.

That split keeps spatial work, policy work, and execution state independent.

## Detection Model

`Interactor` and `Interactable` stay separate.

- `Interactor` owns search radius, scoring weights, candidate limit, LOS preference, and interaction channels.
- `Interactable` owns target-level priority, anchor offset, LOS policy, and enabled state.
- `InteractionTarget` owns one or more `InteractionSlot`s so a single entity can expose multiple actions.

Supported detection paths:

- `DetectionMode::Proximity`
- `DetectionMode::Picking`
- `DetectionMode::Hybrid`

Hybrid mode uses broad proximity collection plus picking confirmation bias. The crate does not reimplement Bevy’s pointer stack; it reads the existing picking interaction data when picking is enabled in the host app.

## Arbitration Rules

Candidate scoring is additive and intentionally cheap:

```text
score =
    distance_score * distance_weight
  + alignment_score * alignment_weight
  + target_priority * target_priority_weight
  + slot_priority * slot_priority_weight
  + picking_bias
```

Important details:

- distance is normalized against the interactor’s effective max distance
- alignment comes from `InteractorAim` when present
- the target contributes its highest slot priority to candidate ranking
- picking and hybrid candidates receive a configurable source bonus
- results are sorted descending and truncated to `candidate_limit`

This makes it practical to bias toward authored “important” targets without discarding spatial intuition.

## Focus Hysteresis

`FocusedInteraction` is intentionally sticky.

- If a new candidate only beats the current focus by less than `hysteresis`, the current target stays focused.
- Hysteresis is evaluated against the current candidate list, not against historical stale data.
- When focus changes, the crate emits `FocusChanged`.

That prevents prompt flicker in dense scenes where several valid targets stay near-identical frame to frame.

## Slot Selection and Prompt Resolution

Once a target is focused, `Gate` chooses one slot.

- Explicit slot selection from `InteractionIntentKind::SelectSlot` wins.
- Slot cycling uses descending slot priority order.
- Without explicit selection, the highest-priority available slot wins.
- If every slot is blocked, the highest-priority slot is still offered with an availability reason.

`InteractionPromptState` stores the resolved `InteractionOffer`, which includes:

- target entity
- slot id
- optional stage id
- prompt metadata
- availability reason
- suppression flag
- focus source

That lets prompt UI stay data-driven while still surfacing blocked actions.

## Gating Rules

`evaluate_slot` checks availability in this order:

1. target or slot enabled state
2. channel compatibility
3. reservations
4. one-shot consumption
5. shared and per-actor cooldowns
6. required and blocked actor tags
7. required and blocked target tags
8. effective distance
9. line of sight, when enabled
10. registered predicate callbacks

Failures return structured `InteractionAvailabilityReason` values instead of a single boolean.

## Execution State Machine

`Execute` owns runtime state in `ActiveInteraction`.

Supported executions:

- `Instant`
- `Hold`
- `Toggle`
- `Mash`
- `Passive`
- staged `Sequence` behavior via `InteractionBehavior::Sequence`

Execution rules:

- `Instant` completes immediately in the start frame.
- `Hold`, `Mash`, and `Passive` create `ActiveInteraction` and advance progress each frame.
- `Toggle` flips per-slot state and completes immediately.
- sequences store their current stage index in runtime state and emit `InteractionStageAdvanced` when they move forward

Accessibility transforms are applied before execution:

- `hold_to_toggle` converts hold slots into immediate toggle-like completions
- `mash_auto_complete` converts mash slots into passive timed progress

## Cancellation Rules

`cancel_active_if_needed` can stop an active interaction because of:

- explicit cancel intent
- release during a hold
- focus loss
- distance break
- line-of-sight break
- reservation loss or other blocked-state re-evaluation
- externally injected cancellation through `InteractionExternalCancel`

Cancellation policies are slot-specific via `InteractionCancelPolicy`.

## Message Flow

Typical flow for a successful interaction:

1. input adapter writes `InteractionIntent`
2. detection and focus resolve `FocusedInteraction`
3. gating writes `InteractionPromptState` and emits `InteractionOffered`
4. execution emits `InteractionStarted`
5. timed interactions emit zero or more `InteractionProgress`
6. completion emits `InteractionCompleted`

Other branches:

- focus changes emit `FocusChanged`
- blocked start attempts emit `InteractionFailed`
- canceled active interactions emit `InteractionCanceled`
- staged interactions emit `InteractionStageAdvanced`

## Prompt Rendering Stays Decoupled

The crate only publishes prompt facts.

- `InteractionPromptState` is the current stable prompt snapshot for a given interactor.
- `InteractionOffered` is the transient message surface for audio, UI, analytics, or tutorials.
- `InteractionFocusedBy` is a lightweight marker for highlights and outlines.

The runtime never instantiates HUD nodes, world widgets, outline materials, or input glyphs on its own. That keeps the shared crate reusable across projects with different UI stacks and presentation standards.
