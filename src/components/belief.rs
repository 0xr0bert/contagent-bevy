use crate::components::identifiers::UUID;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Component, Serialize, Deserialize, Debug, Clone, Default)]
#[require(UUID)]
pub struct Belief {
    pub relationships: EntityHashMap<f64>,
    pub perceptions: EntityHashMap<f64>,
}

#[derive(Bundle)]
pub struct BeliefBundle(pub Belief, pub Name, pub UUID);

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
pub struct Beliefs(pub Vec<BeliefData>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeliefData {
    pub uuid: UUID,
    pub name: String,
    pub relationships: HashMap<String, f64>,
    pub perceptions: HashMap<String, f64>,
}

pub fn spawn_beliefs(mut commands: Commands, beliefs: Res<Beliefs>) {
    for data in beliefs.0.iter() {
        commands.spawn(BeliefBundle(
            Belief::default(),
            Name::new(data.name.clone()),
            data.uuid.clone(),
        ));
    }
}

pub fn setup_belief_links(
    mut query: Query<(Entity, &mut Belief, &UUID)>,
    all_uuids: Query<(Entity, &UUID)>,
    beliefs_data: Res<Beliefs>,
) {
    let uuid_to_entity: HashMap<uuid::Uuid, Entity> =
        all_uuids.iter().map(|(e, u)| (u.0, e)).collect();

    for (_entity, mut belief, uuid) in query.iter_mut() {
        // Find the original data for this belief
        if let Some(data) = beliefs_data.0.iter().find(|d| d.uuid.0 == uuid.0) {
            // Link relationships
            for (target_uuid_str, &weight) in &data.relationships {
                if let Ok(target_uuid) = target_uuid_str.parse::<uuid::Uuid>() {
                    if let Some(&target_entity) = uuid_to_entity.get(&target_uuid) {
                        belief.relationships.insert(target_entity, weight);
                    }
                }
            }

            // Link perceptions
            for (target_uuid_str, &weight) in &data.perceptions {
                if let Ok(target_uuid) = target_uuid_str.parse::<uuid::Uuid>() {
                    if let Some(&target_entity) = uuid_to_entity.get(&target_uuid) {
                        belief.perceptions.insert(target_entity, weight);
                    }
                }
            }
        }
    }
}
