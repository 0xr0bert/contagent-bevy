use crate::components::identifiers::UUID;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;

#[derive(Component)]
#[require(UUID)]
pub struct Belief {
    pub relationships: EntityHashMap<f64>,
    pub perceptions: EntityHashMap<f64>,
}

#[derive(Bundle)]
pub struct BeliefBundle(pub Belief, pub Name);
