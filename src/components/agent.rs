use bevy::ecs::entity::EntityHashMap;
use crate::components::identifiers::UUID;
use bevy::prelude::*;

#[derive(Component)]
#[require(UUID)]
pub struct Agent {
    pub activations: Vec<EntityHashMap<f64>>,
    pub friends: EntityHashMap<f64>,
    pub actions: Vec<Entity>,
    pub deltas: EntityHashMap<f64>,
    pub performance_relationships: EntityHashMap<EntityHashMap<f64>>,
}