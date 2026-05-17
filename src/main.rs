mod components;
pub mod json;
pub mod queries;
pub mod resources;

use bevy::prelude::*;
use bevy_rand::prelude::{EntropyPlugin, WyRand};
use clap::Parser;
use components::agent::{setup_agent_links, spawn_agents};
use components::behaviour::spawn_behaviours;
use components::belief::{setup_belief_links, spawn_beliefs};
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

    for _ in args.start_tick..=args.end_tick {
        app.update();
    }
}

fn increment_tick(mut sim_time: ResMut<SimulationTime>) {
    sim_time.0 += 1;
}
