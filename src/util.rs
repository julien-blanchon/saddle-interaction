use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::components::{
    DetectionMode, InteractionBehavior, InteractionCancelReason, InteractionChannel,
    InteractionExecution, InteractionOccluder, InteractionOccluderShape, InteractionSlot,
    InteractionSlotId, InteractionStage, InteractionStageId,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TargetSlotKey {
    pub target: Entity,
    pub slot_id: String,
}

impl TargetSlotKey {
    pub(crate) fn new(target: Entity, slot_id: &InteractionSlotId) -> Self {
        Self {
            target,
            slot_id: slot_id.0.clone(),
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct SpatialHashIndex {
    pub cell_size: f32,
    pub cells: HashMap<IVec3, Vec<Entity>>,
}

#[derive(Resource, Clone, Default)]
pub(crate) struct InteractionRuntimeState {
    pub active: bool,
    pub shared_cooldowns: HashMap<TargetSlotKey, f64>,
    pub per_actor_cooldowns: HashMap<(Entity, TargetSlotKey), f64>,
    pub consumed_global: HashSet<TargetSlotKey>,
    pub consumed_per_actor: HashSet<(Entity, TargetSlotKey)>,
    pub reservations: HashMap<TargetSlotKey, Entity>,
    pub toggle_states: HashMap<TargetSlotKey, bool>,
    pub stage_indices: HashMap<TargetSlotKey, usize>,
    pub pending_external_cancels: HashMap<Entity, InteractionCancelReason>,
}

pub(crate) fn stage_for_slot<'a>(
    runtime: &'a InteractionRuntimeState,
    target: Entity,
    slot: &'a InteractionSlot,
) -> (Option<&'a InteractionStage>, usize) {
    match &slot.behavior {
        InteractionBehavior::Single(_) => (None, 0),
        InteractionBehavior::Sequence { stages, .. } => {
            if stages.is_empty() {
                return (None, 0);
            }
            let key = TargetSlotKey::new(target, &slot.id);
            let index = runtime
                .stage_indices
                .get(&key)
                .copied()
                .unwrap_or(0)
                .min(stages.len().saturating_sub(1));
            (stages.get(index), index)
        }
    }
}

pub(crate) fn execution_for_slot(
    runtime: &InteractionRuntimeState,
    target: Entity,
    slot: &InteractionSlot,
) -> (InteractionExecution, Option<InteractionStageId>, usize) {
    match &slot.behavior {
        InteractionBehavior::Single(execution) => (*execution, None, 0),
        InteractionBehavior::Sequence { .. } => {
            let (stage, index) = stage_for_slot(runtime, target, slot);
            let stage = stage.expect("sequence stages should be non-empty");
            (stage.execution, Some(stage.id.clone()), index)
        }
    }
}

pub(crate) fn prompt_for_slot(
    runtime: &InteractionRuntimeState,
    target: Entity,
    slot: &InteractionSlot,
) -> (
    crate::components::InteractionPrompt,
    Option<InteractionStageId>,
    usize,
) {
    match &slot.behavior {
        InteractionBehavior::Single(_) => (slot.prompt.clone(), None, 0),
        InteractionBehavior::Sequence { .. } => {
            let (stage, index) = stage_for_slot(runtime, target, slot);
            let stage = stage.expect("sequence stages should be non-empty");
            (
                stage.prompt.clone().unwrap_or_else(|| slot.prompt.clone()),
                Some(stage.id.clone()),
                index,
            )
        }
    }
}

pub(crate) fn matches_channel(
    interactor_channels: &[InteractionChannel],
    target_channels: &[InteractionChannel],
) -> bool {
    if interactor_channels.is_empty() || target_channels.is_empty() {
        return true;
    }

    interactor_channels
        .iter()
        .any(|interactor| target_channels.iter().any(|target| target == interactor))
}

pub(crate) fn effective_detection_mode(
    override_mode: Option<DetectionMode>,
    config_mode: DetectionMode,
) -> DetectionMode {
    override_mode.unwrap_or(config_mode)
}

pub(crate) fn segment_blocked(
    world: &mut World,
    origin: Vec3,
    target: Vec3,
    ignored_entity: Entity,
) -> bool {
    let direction = target - origin;
    if direction.length_squared() <= f32::EPSILON {
        return false;
    }

    let mut q_occluders = world.query::<(Entity, &InteractionOccluder, &GlobalTransform)>();
    for (entity, occluder, transform) in q_occluders.iter(world) {
        if entity == ignored_entity {
            continue;
        }

        let center = transform.translation();
        let blocked = match &occluder.shape {
            InteractionOccluderShape::Sphere { radius } => {
                segment_intersects_sphere(origin, target, center, *radius)
            }
            InteractionOccluderShape::Circle2d { radius } => segment_intersects_sphere(
                Vec3::new(origin.x, origin.y, 0.0),
                Vec3::new(target.x, target.y, 0.0),
                Vec3::new(center.x, center.y, 0.0),
                *radius,
            ),
            InteractionOccluderShape::Cuboid { half_extents } => segment_intersects_aabb(
                origin,
                target,
                center - *half_extents,
                center + *half_extents,
            ),
            InteractionOccluderShape::Rect2d { half_extents } => segment_intersects_aabb(
                Vec3::new(origin.x, origin.y, 0.0),
                Vec3::new(target.x, target.y, 0.0),
                Vec3::new(center.x - half_extents.x, center.y - half_extents.y, -0.1),
                Vec3::new(center.x + half_extents.x, center.y + half_extents.y, 0.1),
            ),
        };

        if blocked {
            return true;
        }
    }

    false
}

fn segment_intersects_sphere(start: Vec3, end: Vec3, center: Vec3, radius: f32) -> bool {
    let segment = end - start;
    let segment_length_sq = segment.length_squared();
    if segment_length_sq <= f32::EPSILON {
        return start.distance_squared(center) <= radius * radius;
    }

    let t = ((center - start).dot(segment) / segment_length_sq).clamp(0.0, 1.0);
    let closest = start + segment * t;
    closest.distance_squared(center) <= radius * radius
}

fn segment_intersects_aabb(start: Vec3, end: Vec3, min: Vec3, max: Vec3) -> bool {
    let direction = end - start;
    let mut t_min: f32 = 0.0;
    let mut t_max: f32 = 1.0;

    for axis in 0..3 {
        let start_axis = start[axis];
        let dir_axis = direction[axis];
        let min_axis = min[axis];
        let max_axis = max[axis];

        if dir_axis.abs() <= f32::EPSILON {
            if start_axis < min_axis || start_axis > max_axis {
                return false;
            }
            continue;
        }

        let inv = 1.0 / dir_axis;
        let mut near = (min_axis - start_axis) * inv;
        let mut far = (max_axis - start_axis) * inv;
        if near > far {
            std::mem::swap(&mut near, &mut far);
        }
        t_min = t_min.max(near);
        t_max = t_max.min(far);
        if t_min > t_max {
            return false;
        }
    }

    true
}

pub(crate) fn resolve_aim_direction(
    aim: Option<Vec3>,
    transform: &GlobalTransform,
) -> Option<Vec3> {
    aim.filter(|value| value.length_squared() > f32::EPSILON)
        .map(Vec3::normalize)
        .or_else(|| {
            let forward = transform.forward().as_vec3();
            (forward.length_squared() > f32::EPSILON).then_some(forward.normalize())
        })
}
