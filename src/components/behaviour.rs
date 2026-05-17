use crate::components::identifiers::UUID;
use bevy::prelude::*;

#[derive(Component)]
#[require(UUID)]
pub struct Behaviour;

#[derive(Bundle)]
pub struct BehaviourBundle(pub Behaviour, pub Name);
