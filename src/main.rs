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
use components::identifiers::Uuid;
use json::{load_agents_from_zstd_json, load_behaviours_from_json, load_beliefs_from_json, SummarySpec};
use queries::agent::{perform_actions, update_activations_for_all_agents_and_beliefs};
use resources::time::SimulationTime;
use resources::seed::Seed;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start tick for the simulation
    start_tick: usize,

    /// End tick for the simulation (exclusive)
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

    /// Optional RNG seed
    seed: Option<u64>,
}

fn main() {
    let args = Args::parse();

    let agents_initial = load_agents_from_zstd_json(&args.agents_path);
    let beliefs = load_beliefs_from_json(&args.beliefs_path);
    let behaviours = load_behaviours_from_json(&args.behaviours_path);

    let seed_value = args.seed.unwrap_or_else(|| rand::random::<u64>());
    let mut app = setup_app(&args, agents_initial, beliefs, behaviours, seed_value);

    // 1. Run Startup systems to spawn and link everything
    app.world_mut().run_schedule(Startup);

    // 2. Priming step: performActions(startTime - 1)
    {
        let priming_tick = args.start_tick - 1;
        use bevy::ecs::system::SystemState;
        let mut system_state: SystemState<(
            Query<(&mut Agent, &Uuid)>,
            Query<Entity, With<components::belief::Belief>>,
            Query<Entity, With<components::behaviour::Behaviour>>,
            Res<Seed>,
        )> = SystemState::new(app.world_mut());
        
        let (mut agent_query, belief_query, behaviour_query, seed) = system_state.get_mut(app.world_mut());
        
        info!("[time={}] Priming actions", priming_tick);
        let beliefs_vec: Vec<Entity> = belief_query.iter().collect();
        let behaviours_vec: Vec<Entity> = behaviour_query.iter().collect();
        let base_seed = seed.0.unwrap_or(0);

        agent_query.par_iter_mut().for_each(|(mut agent, uuid)| {
            let inner_rng = wyrand::WyRand::new((uuid.0.as_u128() ^ (priming_tick as u128) ^ (base_seed as u128)) as u64);
            let mut rng = WyRand::new(inner_rng);
            queries::agent::perform_action(&mut agent, &beliefs_vec, &behaviours_vec, &mut rng, priming_tick);
        });
        
        system_state.apply(app.world_mut());
    }

    // 3. Main simulation loop: for (t in startTime until endTime)
    for _ in args.start_tick..args.end_tick {
        // Runs systems in Update schedule
        app.world_mut().run_schedule(Update);
    }

    handle_output(&args, &mut app);
}

fn setup_app(args: &Args, agents: Agents, beliefs: components::belief::Beliefs, behaviours: components::behaviour::Behaviours, seed_value: u64) -> App {
    let mut app = App::new();

    app.add_plugins((
        MinimalPlugins,
        bevy::log::LogPlugin::default(),
    ));

    app.insert_resource(Seed(Some(seed_value)));
    app.add_plugins(EntropyPlugin::<WyRand>::with_seed(seed_value.to_le_bytes()));

    app.insert_resource(SimulationTime(args.start_tick))
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
            update_activations_for_all_agents_and_beliefs,
            perform_actions,
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
        let summaries = generate_summaries(&args, world, &entity_to_uuid);
        json::save_summaries_to_zstd_json(&args.output, &summaries);
    }
}

fn build_entity_to_uuid_map(world: &mut World) -> HashMap<Entity, String> {
    let mut map = HashMap::new();
    let mut query = world.query::<(Entity, &Uuid)>();
    for (entity, uuid) in query.iter(world) {
        map.insert(entity, uuid.0.to_string());
    }
    map
}

fn collect_final_agents(world: &mut World, entity_to_uuid: &HashMap<Entity, String>) -> Vec<AgentData> {
    let mut agent_query = world.query::<(&Agent, &Uuid)>();
    
    agent_query.iter(world).map(|(agent, uuid)| {
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

        AgentData {
            uuid: uuid.clone(),
            actions,
            activations,
            deltas,
            friends,
            performance_relationships,
        }
    }).collect()
}

fn generate_summaries(args: &Args, world: &mut World, entity_to_uuid: &HashMap<Entity, String>) -> Vec<SummarySpec> {
    let mut agent_query = world.query::<&Agent>();
    let agents: Vec<&Agent> = agent_query.iter(world).collect();

    if agents.is_empty() {
        return Vec::new();
    }

    // Match Kotlin: summaries for startTime until endTime
    (args.start_tick..args.end_tick).map(|t| {
        let mut mean_activations = HashMap::new();
        let mut sd_activations = HashMap::new();
        let mut median_activations = HashMap::new();
        let mut nonzero_activations = HashMap::new();
        let mut n_performers = HashMap::new();

        let belief_entities: HashSet<Entity> = agents.iter()
            .filter_map(|a| a.activations.get(t))
            .flat_map(|layer| layer.keys().copied())
            .collect();

        for belief_entity in belief_entities {
            if let Some(uuid_str) = entity_to_uuid.get(&belief_entity) {
                let mut values: Vec<f64> = agents.iter()
                    .filter_map(|a| a.activations.get(t).and_then(|layer| layer.get(&belief_entity)).copied())
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

                    values.sort_by(|a, b| a.partial_cmp(b).expect("NaN in activations"));
                    let median = if values.len().is_multiple_of(2) {
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
            if let Some(action_entity) = agent.actions.get(t)
                && let Some(uuid_str) = entity_to_uuid.get(action_entity)
            {
                *n_performers.entry(uuid_str.clone()).or_insert(0) += 1;
            }
        }

        SummarySpec {
            mean_activations,
            sd_activations,
            median_activations,
            nonzero_activations,
            n_performers,
        }
    }).collect()
}

fn increment_tick(mut sim_time: ResMut<SimulationTime>) {
    sim_time.0 += 1;
}
