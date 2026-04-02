# saddle-interaction configuration

This crate exposes tuning at four layers:

- global defaults through `InteractionConfig`
- per-actor tuning through `Interactor`
- per-target tuning through `Interactable`
- per-slot tuning through `InteractionSlot` and its nested config types

## Global Defaults

`InteractionConfig` is the runtime-wide default surface.

| Field | Type | Default | Valid range | Effect | Tuning notes |
| --- | --- | --- | --- | --- | --- |
| `detection_mode` | `DetectionMode` | `Proximity` | `Picking`, `Proximity`, `Hybrid` | Selects the default candidate collection backend | Use `Hybrid` in cluttered first-person scenes where proximity alone feels noisy |
| `default_max_distance` | `f32` | `4.0` | `> 0` | Fallback interaction range when an interactor does not override it | Increase for 3D worlds with larger authored units |
| `default_proximity_radius` | `f32` | `3.0` | `> 0` | Fallback coarse candidate radius | In dense scenes, keep this tighter than `default_max_distance` |
| `default_candidate_limit` | `usize` | `8` | `>= 1` | Truncates ranked candidates per interactor | Lower values reduce hot-loop work in object-dense areas |
| `hysteresis` | `f32` | `0.15` | `>= 0` | Minimum score margin before focus can swap to a new target | Raise slightly if prompts flicker between near-equal candidates |
| `default_input_buffer_seconds` | `f32` | `0.12` | `>= 0` | Keeps a confirm press alive briefly so start logic can consume it after focus resolves | Slightly larger values feel better with slower camera movement or gamepad aim |
| `hold_time_scale` | `f32` | `1.0` | `> 0` | Scales all hold durations globally | Lower for accessibility modes, higher for deliberate vulnerability windows |
| `detection_radius_scale` | `f32` | `1.0` | `> 0` | Multiplies effective proximity radius globally | Useful for accessibility assist modes or small-screen couch play |
| `hold_to_toggle` | `bool` | `false` | `bool` | Converts hold slots into immediate completions | Primary accessibility switch for players who cannot sustain holds |
| `mash_auto_complete` | `bool` | `false` | `bool` | Converts mash slots into passive timed completion | Use when you need alternatives to repeated inputs |
| `auto_interact_on_focus` | `bool` | `false` | `bool` | Starts eligible slots automatically on focus | Good for tutorial, onboarding, or fully automatic accessibility flows |

## Interactor

`Interactor` lets each actor override search and ranking behavior.

| Field | Type | Default | Valid range | Effect | Tuning notes |
| --- | --- | --- | --- | --- | --- |
| `detection_mode` | `Option<DetectionMode>` | `None` | same as global | Per-actor override for candidate collection backend | Use for mode switching such as on-foot vs inspection vs build mode |
| `max_distance` | `Option<f32>` | `None` | `> 0` | Per-actor interaction range override | Keep consistent with camera scale and authored prop density |
| `proximity_radius` | `Option<f32>` | `None` | `> 0` | Per-actor broadphase radius override | Tight radii help avoid noisy interact prompts near crowds |
| `candidate_limit` | `Option<usize>` | `None` | `>= 1` | Per-actor candidate truncation | Lower on AI helpers or NPCs that only need the best few options |
| `hysteresis` | `Option<f32>` | `None` | `>= 0` | Per-actor focus stickiness override | Raise for controller-driven actors with slower reticle motion |
| `distance_weight` | `f32` | `1.0` | any finite | Weight of normalized distance in candidate score | Increase when closest-object expectations matter more than authored priority |
| `alignment_weight` | `f32` | `0.35` | any finite | Weight of `InteractorAim` alignment in score | Raise for camera-forward interaction styles |
| `target_priority_weight` | `f32` | `1.0` | any finite | Weight of `Interactable::priority` | Use to make authored “important” props beat nearby clutter |
| `slot_priority_weight` | `f32` | `0.45` | any finite | Weight of the target’s best slot priority | Useful when one object exposes both core and optional interactions |
| `picking_bias` | `f32` | `0.75` | any finite | Bonus applied to picking or hybrid candidates | Raise when pointer-confirmed hits should dominate pure proximity |
| `require_line_of_sight` | `bool` | `false` | `bool` | Default LOS requirement for targets using `UseInteractorSetting` | Enable for first-person or tactical interactions that should respect cover |
| `channels` | `Vec<InteractionChannel>` | `["world"]` | any list | Restricts target compatibility by channel | Use channels to isolate gameplay, editor, photo-mode, or debug interactions |
| `busy_policy` | `InteractorBusyPolicy` | `SingleActive` | `SingleActive` | Keeps the actor in a single-active interaction model | This runtime currently supports one active interaction per actor; use tags or predicates for richer busy-state gating |

## Interactable

`Interactable` controls target-level tuning shared by all its slots.

| Field | Type | Default | Valid range | Effect | Tuning notes |
| --- | --- | --- | --- | --- | --- |
| `enabled` | `bool` | `true` | `bool` | Disables the entire target | Good for scripted setup or streaming visibility gates |
| `focus_radius` | `Option<f32>` | `None` | `> 0` | Target-specific proximity radius | Use for large props or forgiving interaction volumes |
| `priority` | `f32` | `0.0` | any finite | Target-level authored priority bonus | Increase for “hero” props that should beat nearby clutter |
| `channels` | `Vec<InteractionChannel>` | `["world"]` | any list | Restricts which interactors may see the target | Separate debug tools or build-mode targets from regular gameplay |
| `line_of_sight_policy` | `LineOfSightPolicy` | `UseInteractorSetting` | enum | Controls LOS evaluation for the target | Force `Require` on precise world props like terminals or wall switches |
| `anchor_entity` | `Option<Entity>` | `None` | entity id | Optional anchor source for prompt or highlight placement | Useful when the interactable root is not the visual point of interest |
| `anchor_offset` | `Vec3` | `Vec3::ZERO` | any finite | World-space offset for scoring and prompt anchoring | Raise anchors for tall props so prompts sit above the object |

## Slots

`InteractionSlot` is the core authored action surface.

| Field | Type | Default | Valid range | Effect | Tuning notes |
| --- | --- | --- | --- | --- | --- |
| `id` | `InteractionSlotId` | `"default"` | stable string-like id | Public slot identity used by prompts, saves, analytics, and consumers | Keep stable across content revisions |
| `prompt` | `InteractionPrompt` | see below | semantic prompt data | Carries label, hint, icon, and anchor metadata | Keep physical key glyphs out of this field; use semantic hint keys instead |
| `behavior` | `InteractionBehavior` | `Single(Instant)` | single or sequence | Chooses how the slot executes | Use sequences for staged interactions without creating multiple entities |
| `availability` | `InteractionAvailabilityConfig` | enabled reusable slot | structured gating | Defines tags, predicates, and one-shot consumption | Prefer tags for simple gating and predicates for rare custom checks |
| `priority` | `f32` | `0.0` | any finite | Slot-level authored priority | Use to make one slot the default choice on a multi-action target |
| `auto_trigger_on_focus` | `bool` | `false` | `bool` | Starts the slot automatically when it becomes focused and available | Useful for tutorials or accessibility-first flows |
| `cooldown` | `InteractionCooldown` | zeroed | seconds | Shared, per-actor, and acceptance-delay cooldown settings | Add short acceptance delays to prevent accidental double-triggering |
| `cancellation` | `InteractionCancelPolicy` | all true | booleans | Controls which interruptions cancel active execution | Relax `on_focus_loss` for forgiving interaction sequences |
| `reservation` | `InteractionReservationPolicy` | `Shared` | enum | Determines whether another actor may start the same slot concurrently | Use `Exclusive` on enter-vehicle or use-terminal flows |

### Prompt Surface

`InteractionPrompt` fields:

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `prompt_key` | `String` | `"interaction.prompt"` | Top-level localisation key or prompt template identifier |
| `action_label_key` | `String` | `"interaction.action"` | Semantic action label shown to players |
| `input_hint_key` | `Option<String>` | `None` | Optional semantic hint key, not a hardcoded physical button string |
| `icon_key` | `Option<String>` | `None` | Optional icon identifier for HUD or diegetic widgets |
| `priority` | `i32` | `0` | Prompt-layer ordering hint for UI systems |
| `suppression` | `PromptSuppressionPolicy` | `Never` | Controls whether unavailable prompts should hide |
| `anchor_entity` | `Option<Entity>` | `None` | Optional prompt anchor override |
| `anchor_offset` | `Vec3` | `Vec3::ZERO` | Prompt offset relative to the chosen anchor |

### Availability Surface

`InteractionAvailabilityConfig` fields:

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `enabled` | `bool` | `true` | Disables the slot entirely |
| `required_actor_tags` | `Vec<InteractionTag>` | empty | Actor must contain every listed tag |
| `blocked_actor_tags` | `Vec<InteractionTag>` | empty | Actor must not contain any listed tag |
| `required_target_tags` | `Vec<InteractionTag>` | empty | Target must contain every listed tag |
| `blocked_target_tags` | `Vec<InteractionTag>` | empty | Target must not contain any listed tag |
| `predicate_ids` | `Vec<InteractionPredicateId>` | empty | Custom callback ids resolved through `InteractionPredicateRegistry` |
| `consumption` | `InteractionConsumption` | `Reusable` | Configures one-shot global or per-actor consumption |

### Cooldown Surface

`InteractionCooldown` fields:

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `shared_seconds` | `f32` | `0.0` | Blocks all actors from reusing the slot for a duration |
| `per_actor_seconds` | `f32` | `0.0` | Blocks only the same actor from reusing the slot |
| `acceptance_delay_seconds` | `f32` | `0.0` | Reserved surface for post-acceptance delay or debounce-style flows |

### Cancellation Surface

`InteractionCancelPolicy` fields:

| Field | Type | Default | Effect |
| --- | --- | --- | --- |
| `on_release` | `bool` | `true` | Releasing confirm cancels active holds |
| `on_focus_loss` | `bool` | `true` | Losing focus cancels the active slot |
| `on_distance_break` | `bool` | `true` | Intended authoring flag for range-break cancellation |
| `on_line_of_sight_break` | `bool` | `true` | Intended authoring flag for LOS-break cancellation |
| `on_blocked_state` | `bool` | `true` | Re-evaluates gating while active and cancels when the slot becomes invalid |

## Execution Shapes

`InteractionExecution` supports:

- `Instant`
- `Hold { duration_seconds }`
- `Toggle`
- `Mash { required_presses, decay_per_second }`
- `Passive { duration_seconds }`

`InteractionBehavior::Sequence` wraps staged execution:

- `stages: Vec<InteractionStage>`
- `advance_mode: SequenceAdvanceMode`

## Accessibility Tuning

- Start with `hold_to_toggle = true` before widening raw hold durations. It keeps authored timings intact while removing sustained-input pressure.
- Use `detection_radius_scale > 1.0` before increasing every authored target radius individually.
- If mash sequences feel exclusionary, prefer `mash_auto_complete = true` plus a readable prompt rather than inventing a second slot.

## Dense-Scene Tuning

- Lower `default_candidate_limit` first.
- Increase `hysteresis` slightly.
- Raise `target_priority_weight` only on authored hero props, not globally on every interactor.
- Use channels to remove non-gameplay interactables from the same ranking pool.

## Mouse and Controller Tuning

- Mouse or cursor-driven games usually benefit from `DetectionMode::Hybrid` and a healthy `picking_bias`.
- Controller-centric third-person or first-person games usually benefit from higher `alignment_weight`.
- If controller users struggle to keep prompts stable, raise `hysteresis` before inflating `max_distance`.
