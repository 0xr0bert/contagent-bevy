use crate::components::identifiers::Uuid;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Debug, Clone)]
#[require(Uuid)]
pub struct Behaviour;

#[derive(Bundle)]
pub struct BehaviourBundle(pub Behaviour, pub Name, pub Uuid);

#[derive(Resource, Serialize, Deserialize, Debug, Clone, Deref)]
pub struct Behaviours(pub Vec<BehaviourData>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BehaviourData {
    pub name: String,
    pub uuid: Uuid,
}

pub fn spawn_behaviours(mut commands: Commands, behaviours: Res<Behaviours>) {
    for data in behaviours.0.iter() {
        commands.spawn(BehaviourBundle(
            Behaviour,
            Name::new(data.name.clone()),
            data.uuid.clone(),
        ));
    }
}
