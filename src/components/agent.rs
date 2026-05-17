use crate::components::identifiers::Uuid;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Component, Serialize, Deserialize, Debug, Clone, Default)]
#[require(Uuid)]
pub struct Agent {
    pub activations: Vec<EntityHashMap<f64>>,
    pub friends: EntityHashMap<f64>,
    pub actions: Vec<Entity>,
    pub deltas: EntityHashMap<f64>,
    pub performance_relationships: EntityHashMap<EntityHashMap<f64>>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
pub struct Agents(pub Vec<AgentData>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentData {
    pub uuid: Uuid,
    pub actions: Vec<String>,
    pub activations: Vec<HashMap<String, f64>>,
    pub deltas: HashMap<String, f64>,
    pub friends: HashMap<String, f64>,
    pub performance_relationships: HashMap<String, HashMap<String, f64>>,
}

pub fn spawn_agents(mut commands: Commands, agents: Res<Agents>) {
    for data in agents.0.iter() {
        commands.spawn((
            Agent::default(),
            Name::new(format!("Agent-{}", data.uuid.0)),
            data.uuid.clone(),
        ));
    }
}

pub fn setup_agent_links(
    mut query: Query<(&mut Agent, &Uuid)>,
    all_uuids: Query<(Entity, &Uuid)>,
    agents_data: Res<Agents>,
) {
    let uuid_to_entity: HashMap<uuid::Uuid, Entity> =
        all_uuids.iter().map(|(e, u)| (u.0, e)).collect();

    for (mut agent, uuid) in query.iter_mut() {
        if let Some(data) = agents_data.0.iter().find(|d| d.uuid.0 == uuid.0) {
            // Link actions
            for action_uuid_str in &data.actions {
                if let Ok(action_uuid) = action_uuid_str.parse::<uuid::Uuid>()
                    && let Some(&target_entity) = uuid_to_entity.get(&action_uuid)
                {
                    agent.actions.push(target_entity);
                }
            }

            // Link activations
            for layer in &data.activations {
                let mut entity_layer = EntityHashMap::default();
                for (target_uuid_str, &value) in layer {
                    if let Ok(target_uuid) = target_uuid_str.parse::<uuid::Uuid>()
                        && let Some(&target_entity) = uuid_to_entity.get(&target_uuid)
                    {
                        entity_layer.insert(target_entity, value);
                    }
                }
                agent.activations.push(entity_layer);
            }

            // Link deltas
            for (target_uuid_str, &value) in &data.deltas {
                if let Ok(target_uuid) = target_uuid_str.parse::<uuid::Uuid>()
                    && let Some(&target_entity) = uuid_to_entity.get(&target_uuid)
                {
                    agent.deltas.insert(target_entity, value);
                }
            }

            // Link friends
            for (target_uuid_str, &value) in &data.friends {
                if let Ok(target_uuid) = target_uuid_str.parse::<uuid::Uuid>()
                    && let Some(&target_entity) = uuid_to_entity.get(&target_uuid)
                {
                    agent.friends.insert(target_entity, value);
                }
            }

            // Link performance relationships
            for (source_uuid_str, targets) in &data.performance_relationships {
                if let Ok(source_uuid) = source_uuid_str.parse::<uuid::Uuid>()
                    && let Some(&source_entity) = uuid_to_entity.get(&source_uuid)
                {
                    let mut target_map = EntityHashMap::default();
                    for (target_uuid_str, &value) in targets {
                        if let Ok(target_uuid) = target_uuid_str.parse::<uuid::Uuid>()
                            && let Some(&target_entity) = uuid_to_entity.get(&target_uuid)
                        {
                            target_map.insert(target_entity, value);
                        }
                    }
                    agent.performance_relationships.insert(source_entity, target_map);
                }
            }
        }
    }
}
