mod components;
pub mod queries;
pub mod resources;

use bevy::prelude::*;
use bevy_rand::prelude::{EntropyPlugin, WyRand};
use queries::agent::{perform_actions, update_activations_for_all_agents_and_beliefs};
use resources::time::SimulationTime;

const START_TICK: usize = 1;
const END_TICK: usize = 100;

fn main() {
    let mut app = App::new();

    app.add_plugins((MinimalPlugins, EntropyPlugin::<WyRand>::default()))
        .insert_resource(SimulationTime(START_TICK))
        .add_systems(
            Update,
            (
                perform_actions,
                update_activations_for_all_agents_and_beliefs,
                increment_tick,
            )
                .chain(),
        );

    for _ in START_TICK..=END_TICK {
        app.update();
    }
}

fn increment_tick(mut sim_time: ResMut<SimulationTime>) {
    sim_time.0 += 1;
}
