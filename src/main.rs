mod components;
pub mod queries;
pub mod resources;

use bevy::prelude::*;
use bevy_rand::prelude::{EntropyPlugin, WyRand};
use clap::Parser;
use queries::agent::{perform_actions, update_activations_for_all_agents_and_beliefs};
use resources::time::SimulationTime;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Start tick for the simulation
    start_tick: usize,

    /// End tick for the simulation
    end_tick: usize,
}

fn main() {
    let args = Args::parse();

    let mut app = App::new();

    app.add_plugins((MinimalPlugins, EntropyPlugin::<WyRand>::default()))
        .insert_resource(SimulationTime(args.start_tick))
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
