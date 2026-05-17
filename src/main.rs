mod components;
pub mod json;
pub mod queries;
pub mod resources;

use bevy::prelude::*;
use bevy_rand::prelude::{EntropyPlugin, WyRand};
use clap::Parser;
use components::behaviour::spawn_behaviours;
use json::load_behaviours_from_json;
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

    /// Path to the behaviours JSON file
    behaviours_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    // Load behaviours using the new module
    let behaviours = load_behaviours_from_json(&args.behaviours_path);

    let mut app = App::new();

    app.add_plugins((MinimalPlugins, EntropyPlugin::<WyRand>::default()))
        .insert_resource(SimulationTime(args.start_tick))
        .insert_resource(behaviours) // Insert as a resource for the system to use
        .add_systems(Startup, spawn_behaviours)
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
