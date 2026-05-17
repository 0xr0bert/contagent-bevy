mod components;
pub mod json;
pub mod queries;
pub mod resources;

use bevy::prelude::*;
use bevy_rand::prelude::{EntropyPlugin, WyRand};
use clap::Parser;
use components::agent::{spawn_agents, setup_agent_links, Agent};
use components::behaviour::spawn_behaviours;
use components::belief::{setup_belief_links, spawn_beliefs};
use components::identifiers::UUID;
use json::{load_agents_from_zstd_json, load_behaviours_from_json, load_beliefs_from_json};
use queries::agent::{perform_actions, update_activations_for_all_agents_and_beliefs};
use resources::time::SimulationTime;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start tick for the simulation
    start_tick: usize,

    /// End tick for the simulation
    end_tick: usize,

    /// Path to the agents ZSTD-compressed JSON file
    agents_path: PathBuf,

    /// Path to the beliefs JSON file
    beliefs_path: PathBuf,

    /// Path to the behaviours JSON file
    behaviours_path: PathBuf,

    /// Path to the output ZSTD-compressed JSON file
    output: PathBuf,

    /// Whether to output the full agents data
    #[arg(action = clap::ArgAction::Set)]
    full_output: bool,
}

fn main() {
    let args = Args::parse();

    // Load data using the new module
    let agents = load_agents_from_zstd_json(&args.agents_path);
    let beliefs = load_beliefs_from_json(&args.beliefs_path);
    let behaviours = load_behaviours_from_json(&args.behaviours_path);

    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        EntropyPlugin::<WyRand>::default(),
        bevy::log::LogPlugin::default(),
    ))
        .insert_resource(SimulationTime(args.start_tick))
        .insert_resource(agents.clone())
        .insert_resource(beliefs)
        .insert_resource(behaviours)
        .add_systems(
            Startup,
            (
                (spawn_agents, spawn_beliefs, spawn_behaviours),
                (setup_belief_links, setup_agent_links),
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                perform_actions,
                update_activations_for_all_agents_and_beliefs,
                increment_tick,
            )
                .chain(),
        );

    for _ in args.start_tick..=args.end_tick {
        app.update();
    }

    if args.full_output {
        let mut final_agents = Vec::new();
        let world = app.world_mut();
        
        // Build a mapping from Entity to UUID for reverse lookup
        let mut entity_to_uuid = std::collections::HashMap::new();
        let mut uuid_query = world.query::<(Entity, &UUID)>();
        for (entity, uuid) in uuid_query.iter(world) {
            entity_to_uuid.insert(entity, uuid.clone());
        }

        // Query all agents to get their final state
        let mut agent_query = world.query::<(&Agent, &UUID)>();
        for (agent, uuid) in agent_query.iter(world) {
            let mut actions = Vec::new();
            for action_entity in &agent.actions {
                if let Some(action_uuid) = entity_to_uuid.get(action_entity) {
                    actions.push(action_uuid.0.to_string());
                }
            }

            let mut activations = Vec::new();
            for layer in &agent.activations {
                let mut layer_data = std::collections::HashMap::new();
                for (entity, value) in layer {
                    if let Some(u) = entity_to_uuid.get(entity) {
                        layer_data.insert(u.0.to_string(), *value);
                    }
                }
                activations.push(layer_data);
            }

            let mut deltas = std::collections::HashMap::new();
            for (entity, value) in &agent.deltas {
                if let Some(u) = entity_to_uuid.get(entity) {
                    deltas.insert(u.0.to_string(), *value);
                }
            }

            let mut friends = std::collections::HashMap::new();
            for (entity, value) in &agent.friends {
                if let Some(u) = entity_to_uuid.get(entity) {
                    friends.insert(u.0.to_string(), *value);
                }
            }

            let mut performance_relationships = std::collections::HashMap::new();
            for (source_entity, targets) in &agent.performance_relationships {
                if let Some(source_uuid) = entity_to_uuid.get(source_entity) {
                    let mut target_map = std::collections::HashMap::new();
                    for (target_entity, value) in targets {
                        if let Some(target_uuid) = entity_to_uuid.get(target_entity) {
                            target_map.insert(target_uuid.0.to_string(), *value);
                        }
                    }
                    performance_relationships.insert(source_uuid.0.to_string(), target_map);
                }
            }

            final_agents.push(components::agent::AgentData {
                uuid: uuid.clone(),
                actions,
                activations,
                deltas,
                friends,
                performance_relationships,
            });
        }

        json::save_agents_to_zstd_json(&args.output, &components::agent::Agents(final_agents));
    }
}

fn increment_tick(mut sim_time: ResMut<SimulationTime>) {
    sim_time.0 += 1;
}
