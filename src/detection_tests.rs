use bevy::{
    picking::{backend::HitData, pointer::PointerInteraction},
    prelude::*,
    reflect::ReflectMut,
};

use super::*;
use crate::{
    components::{
        DetectionMode, FocusSource, Interactable, InteractionCandidates, InteractionChannel,
        InteractionSlot, InteractionTarget, Interactor, InteractorPointer,
    },
    config::{InteractionConfig, InteractionStats},
    util::{InteractionRuntimeState, SpatialHashIndex},
};

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(InteractionConfig::default());
    app.insert_resource(InteractionStats::default());
    app.insert_resource(InteractionRuntimeState {
        active: true,
        ..default()
    });
    app.insert_resource(SpatialHashIndex::default());
    app.add_message::<crate::messages::InteractionIntent>();
    app.add_message::<crate::messages::InteractionExternalCancel>();
    app.add_systems(
        Update,
        (
            prepare_interactors,
            rebuild_spatial_index,
            collect_candidates,
        )
            .chain(),
    );
    app
}

#[test]
fn detection_filtering_respects_distance_and_channels() {
    let mut app = test_app();
    let interactor = app
        .world_mut()
        .spawn((
            Name::new("Interactor"),
            Interactor {
                max_distance: Some(5.0),
                proximity_radius: Some(5.0),
                channels: vec![InteractionChannel::from("world")],
                ..default()
            },
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    let near_match = app
        .world_mut()
        .spawn((
            Name::new("Near Match"),
            Interactable::default(),
            InteractionTarget {
                slots: vec![InteractionSlot::instant("open", "interaction.open")],
            },
            GlobalTransform::from_xyz(2.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().spawn((
        Name::new("Too Far"),
        Interactable::default(),
        InteractionTarget {
            slots: vec![InteractionSlot::instant("far", "interaction.far")],
        },
        GlobalTransform::from_xyz(8.0, 0.0, 0.0),
    ));
    app.world_mut().spawn((
        Name::new("Wrong Channel"),
        Interactable {
            channels: vec![InteractionChannel::from("ui")],
            ..default()
        },
        InteractionTarget {
            slots: vec![InteractionSlot::instant("ui", "interaction.ui")],
        },
        GlobalTransform::from_xyz(1.0, 0.0, 0.0),
    ));

    app.update();

    let candidates = app
        .world()
        .get::<InteractionCandidates>(interactor)
        .expect("candidates should exist");
    assert_eq!(candidates.entries.len(), 1);
    assert_eq!(candidates.entries[0].target, near_match);
}

#[test]
fn simultaneous_interactors_collect_independent_candidates() {
    let mut app = test_app();
    let first = app
        .world_mut()
        .spawn((
            Name::new("First"),
            Interactor {
                max_distance: Some(4.0),
                proximity_radius: Some(4.0),
                ..default()
            },
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();
    let second = app
        .world_mut()
        .spawn((
            Name::new("Second"),
            Interactor {
                max_distance: Some(4.0),
                proximity_radius: Some(4.0),
                ..default()
            },
            GlobalTransform::from_xyz(10.0, 0.0, 0.0),
        ))
        .id();

    let left_target = app
        .world_mut()
        .spawn((
            Name::new("Left Target"),
            Interactable::default(),
            InteractionTarget {
                slots: vec![InteractionSlot::instant("left", "interaction.left")],
            },
            GlobalTransform::from_xyz(1.5, 0.0, 0.0),
        ))
        .id();
    let right_target = app
        .world_mut()
        .spawn((
            Name::new("Right Target"),
            Interactable::default(),
            InteractionTarget {
                slots: vec![InteractionSlot::instant("right", "interaction.right")],
            },
            GlobalTransform::from_xyz(9.0, 0.0, 0.0),
        ))
        .id();

    app.update();

    assert_eq!(
        app.world()
            .get::<InteractionCandidates>(first)
            .unwrap()
            .entries
            .first()
            .unwrap()
            .target,
        left_target
    );
    assert_eq!(
        app.world()
            .get::<InteractionCandidates>(second)
            .unwrap()
            .entries
            .first()
            .unwrap()
            .target,
        right_target
    );
}

#[test]
fn picking_detection_uses_pointer_hits_from_the_matching_camera() {
    let mut app = test_app();
    let camera = app.world_mut().spawn_empty().id();
    let interactor = app
        .world_mut()
        .spawn((
            Name::new("Pointer Interactor"),
            Interactor {
                detection_mode: Some(DetectionMode::Picking),
                max_distance: Some(5.0),
                ..default()
            },
            InteractorPointer {
                camera: Some(camera),
            },
            GlobalTransform::from_xyz(0.0, 0.0, 0.0),
        ))
        .id();

    let target = app
        .world_mut()
        .spawn((
            Name::new("Pointer Target"),
            Interactable::default(),
            InteractionTarget {
                slots: vec![InteractionSlot::instant("inspect", "interaction.inspect")],
            },
            GlobalTransform::from_xyz(2.0, 0.0, 0.0),
        ))
        .id();

    app.world_mut()
        .spawn(pointer_interaction_with_hit(target, camera));
    app.update();

    let candidates = app
        .world()
        .get::<InteractionCandidates>(interactor)
        .expect("picking candidates should exist");
    assert_eq!(candidates.entries.len(), 1);
    assert_eq!(candidates.entries[0].target, target);
    assert_eq!(candidates.entries[0].source, FocusSource::Picking);
}

fn pointer_interaction_with_hit(target: Entity, camera: Entity) -> PointerInteraction {
    let mut interaction = PointerInteraction::default();
    let ReflectMut::Struct(reflected) = interaction.reflect_mut() else {
        panic!("PointerInteraction should reflect as a struct");
    };
    let hits = reflected
        .field_mut("sorted_entities")
        .expect("PointerInteraction should expose sorted_entities through reflection")
        .try_downcast_mut::<Vec<(Entity, HitData)>>()
        .expect("sorted_entities should be a Vec<(Entity, HitData)>");
    hits.push((
        target,
        HitData::new(camera, 0.1, Some(Vec3::new(2.0, 0.0, 0.0)), None),
    ));
    interaction
}
