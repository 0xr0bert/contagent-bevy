mod components;
pub mod json;
pub mod queries;
pub mod resources;

use bevy::prelude::*;
use bevy_rand::prelude::{EntropyPlugin, WyRand};
use clap::Parser;
use components::agent::{spawn_agents, setup_agent_links, Agent, AgentData, Agents};
use components::behaviour::spawn_behaviours;
use components::belief::{setup_belief_links, spawn_beliefs};
use components::identifiers::UUID;
use json::{load_agents_from_zstd_json, load_behaviours_from_json, load_beliefs_from_json, SummarySpec};
use queries::agent::{perform_actions, update_activations_for_all_agents_and_beliefs};
use resources::time::SimulationTime;
use std::collections::{HashMap, HashSet};
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

    let agents_initial = load_agents_from_zstd_json(&args.agents_path);
    let beliefs = load_beliefs_from_json(&args.beliefs_path);
    let behaviours = load_behaviours_from_json(&args.behaviours_path);

    let mut app = setup_app(&args, agents_initial, beliefs, behaviours);

    for _ in args.start_tick..=args.end_tick {
        app.update();
    }

    handle_output(&args, &mut app);
}

fn setup_app(args: &Args, agents: Agents, beliefs: components::belief::Beliefs, behaviours: components::behaviour::Behaviours) -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        EntropyPlugin::<WyRand>::default(),
        bevy::log::LogPlugin::default(),
    ))
    .insert_resource(SimulationTime(args.start_tick))
    .insert_resource(agents)
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

    app
}

fn handle_output(args: &Args, app: &mut App) {
    let world = app.world_mut();
    let entity_to_uuid = build_entity_to_uuid_map(world);

    if args.full_output {
        let final_agents = collect_final_agents(world, &entity_to_uuid);
        json::save_agents_to_zstd_json(&args.output, &Agents(final_agents));
    } else {
        let summaries = generate_summaries(world, &entity_to_uuid);
        json::save_summaries_to_zstd_json(&args.output, &summaries);
    }
}

fn build_entity_to_uuid_map(world: &mut World) -> HashMap<Entity, String> {
    let mut map = HashMap::new();
    let mut query = world.query::<(Entity, &UUID)>();
    for (entity, uuid) in query.iter(world) {
        map.insert(entity, uuid.0.to_string());
    }
    map
}

fn collect_final_agents(world: &mut World, entity_to_uuid: &HashMap<Entity, String>) -> Vec<AgentData> {
    let mut final_agents = Vec::new();
    let mut agent_query = world.query::<(&Agent, &UUID)>();
    
    for (agent, uuid) in agent_query.iter(world) {
        let actions = agent.actions.iter()
            .filter_map(|e| entity_to_uuid.get(e).cloned())
            .collect();

        let activations = agent.activations.iter().map(|layer| {
            layer.iter()
                .filter_map(|(e, v)| entity_to_uuid.get(e).map(|u| (u.clone(), *v)))
                .collect()
        }).collect();

        let deltas = agent.deltas.iter()
            .filter_map(|(e, v)| entity_to_uuid.get(e).map(|u| (u.clone(), *v)))
            .collect();

        let friends = agent.friends.iter()
            .filter_map(|(e, v)| entity_to_uuid.get(e).map(|u| (u.clone(), *v)))
            .collect();

        let performance_relationships = agent.performance_relationships.iter()
            .filter_map(|(source_e, targets)| {
                entity_to_uuid.get(source_e).map(|source_u| {
                    let target_map = targets.iter()
                        .filter_map(|(target_e, v)| entity_to_uuid.get(target_e).map(|target_u| (target_u.clone(), *v)))
                        .collect();
                    (source_u.clone(), target_map)
                })
            }).collect();

        final_agents.push(AgentData {
            uuid: uuid.clone(),
            actions,
            activations,
            deltas,
            friends,
            performance_relationships,
        });
    }
    final_agents
}

fn generate_summaries(world: &mut World, entity_to_uuid: &HashMap<Entity, String>) -> Vec<SummarySpec> {
    let mut summaries = Vec::new();
    let mut agent_query = world.query::<&Agent>();
    let agents: Vec<&Agent> = agent_query.iter(world).collect();

    if agents.is_empty() {
        return summaries;
    }

    let n_ticks = agents[0].activations.len();
    for t in 0..n_ticks {
        let mut mean_activations = HashMap::new();
        let mut sd_activations = HashMap::new();
        let mut median_activations = HashMap::new();
        let mut nonzero_activations = HashMap::new();
        let mut n_performers = HashMap::new();

        let mut belief_entities = HashSet::new();
        for agent in &agents {
            if let Some(layer) = agent.activations.get(t) {
                for entity in layer.keys() {
                    belief_entities.insert(*entity);
                }
            }
        }

        for belief_entity in belief_entities {
            if let Some(uuid_str) = entity_to_uuid.get(&belief_entity) {
                let mut values: Vec<f64> = agents.iter()
                    .filter_map(|a| a.activations.get(t).and_then(|layer| layer.get(&belief_entity)).cloned())
                    .collect();

                if !values.is_empty() {
                    let n = values.len() as f64;
                    let mean = values.iter().sum::<f64>() / n;
                    mean_activations.insert(uuid_str.clone(), mean);

                    if values.len() > 1 {
                        let variance = values.iter()
                            .map(|v| (v - mean).powi(2))
                            .sum::<f64>() / (n - 1.0);
                        sd_activations.insert(uuid_str.clone(), variance.sqrt());
                    } else {
                        sd_activations.insert(uuid_str.clone(), 0.0);
                    }

                    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let median = if values.len() % 2 == 0 {
                        (values[values.len() / 2 - 1] + values[values.len() / 2]) / 2.0
                    } else {
                        values[values.len() / 2]
                    };
                    median_activations.insert(uuid_str.clone(), median);

                    let nonzero = values.iter().filter(|&&v| v != 0.0).count() as i32;
                    nonzero_activations.insert(uuid_str.clone(), nonzero);
                }
            }
        }

        for agent in &agents {
            if let Some(action_entity) = agent.actions.get(t) {
                if let Some(uuid_str) = entity_to_uuid.get(action_entity) {
                    *n_performers.entry(uuid_str.clone()).or_insert(0) += 1;
                }
            }
        }

        summaries.push(SummarySpec {
            mean_activations,
            sd_activations,
            median_activations,
            nonzero_activations,
            n_performers,
        });
    }
    summaries
}

fn increment_tick(mut sim_time: ResMut<SimulationTime>) {
    sim_time.0 += 1;
}
