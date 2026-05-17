use crate::components::agent::Agent;
use crate::components::behaviour::Behaviour;
use crate::components::belief::Belief;
use crate::components::identifiers::UUID;
use crate::resources::time::SimulationTime;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy_rand::prelude::WyRand;
use itertools::Itertools;
use rand::prelude::*;

use crate::resources::seed::Seed;

pub fn perform_actions(
    mut agent_query: Query<(&mut Agent, &UUID)>,
    belief_query: Query<Entity, With<Belief>>,
    behaviour_query: Query<Entity, With<Behaviour>>,
    sim_time: Res<SimulationTime>,
    seed: Res<Seed>,
) {
    info!("[time={}] Performing actions", sim_time.0);
    let beliefs: Vec<Entity> = belief_query.iter().collect();
    let behaviours: Vec<Entity> = behaviour_query.iter().collect();

    agent_query.par_iter_mut().for_each(|(mut agent, uuid)| {
        // Deterministic seeding per agent and tick, incorporating the global seed if provided
        let base_seed = seed.0.unwrap_or(0);
        let inner_rng = wyrand::WyRand::new((uuid.0.as_u128() ^ (sim_time.0 as u128) ^ (base_seed as u128)) as u64);
        let mut rng = WyRand::new(inner_rng);
        perform_action(&mut agent, &beliefs, &behaviours, &mut rng);
    });
}

fn perform_action(
    agent: &mut Agent,
    beliefs: &[Entity],
    behaviours: &[Entity],
    rng: &mut WyRand,
) {
    let unnormalized_probabilities: Vec<(Entity, f64)> = behaviours
        .iter()
        .map(|behaviour_entity| {
            (
                *behaviour_entity,
                beliefs
                    .iter()
                    .map(|belief_entity| {
                        agent
                            .performance_relationships
                            .get(belief_entity)
                            .expect("Missing performance relationship")
                            .get(behaviour_entity)
                            .unwrap_or(&0.0)
                    })
                    .sum::<f64>(),
            )
        })
        .sorted_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .collect();

    let last_prob = unnormalized_probabilities.last().unwrap();

    if last_prob.1 < 0.0 {
        agent.actions.push(last_prob.0)
    } else {
        let filtered_probs: Vec<(Entity, f64)> = unnormalized_probabilities
            .iter()
            .cloned()
            .filter(|(_, prob)| *prob >= 0.0)
            .collect();
        if filtered_probs.len() == 1 {
            agent.actions.push(filtered_probs[0].0)
        } else {
            let normalizing_factor = filtered_probs
                .as_slice()
                .iter()
                .map(|(_, prob)| prob)
                .sum::<f64>();
            
            if normalizing_factor > 0.0 {
                let normalized_probabilities: Vec<(Entity, f64)> = filtered_probs
                    .into_iter()
                    .map(|(entity, prob)| (entity, prob / normalizing_factor))
                    .collect();
                
                match normalized_probabilities.choose_weighted(rng, |(_, prob)| *prob) {
                    Ok(selected_action) => agent.actions.push(selected_action.0),
                    Err(e) => {
                        warn!("Failed to choose action with weights: {:?}. Defaulting to first behavior.", e);
                        agent.actions.push(unnormalized_probabilities[0].0);
                    }
                }
            } else {
                // If all weights are 0.0, choose one behavior at random
                if let Some(selected) = unnormalized_probabilities.choose(rng) {
                    agent.actions.push(selected.0);
                } else {
                    error!("No behaviors available for action selection!");
                }
            }
        }
    }
}

pub fn update_activations_for_all_agents_and_beliefs(
    mut agent_query: Query<(Entity, &mut Agent)>,
    belief_query: Query<(Entity, &Belief)>,
    sim_time: Res<SimulationTime>,
) {
    info!("[time={}] Perceiving beliefs", sim_time.0);
    let actions: Vec<(Entity, Entity)> = agent_query
        .iter()
        .map(|(entity, agent)| (entity, agent.actions[sim_time.0 - 1]))
        .collect();

    let beliefs: Vec<(Entity, &Belief)> = belief_query.iter().collect();

    agent_query.par_iter_mut().for_each(|(_, mut agent)| {
        update_activations_for_all_belief(&mut agent, &actions, &beliefs, &sim_time);
    });
}

fn update_activations_for_all_belief(
    agent: &mut Agent,
    actions_at_previous_time: &[(Entity, Entity)],
    beliefs: &[(Entity, &Belief)],
    sim_time: &SimulationTime,
) {
    for entry in beliefs.iter() {
        update_activation(
            agent,
            actions_at_previous_time,
            entry,
            beliefs,
            sim_time,
        );
    }
}

fn update_activation(
    agent: &mut Agent,
    actions_at_previous_time: &[(Entity, Entity)],
    belief: &(Entity, &Belief),
    beliefs: &[(Entity, &Belief)],
    sim_time: &SimulationTime,
) {
    let delta = *agent.deltas.get(&belief.0).expect("Missing delta");
    let activations = &agent.activations[sim_time.0 - 1];
    let activation = *activations.get(&belief.0).unwrap_or(&0.0);

    let activation_change_v = activation_change(
        agent,
        actions_at_previous_time,
        belief,
        beliefs,
        &SimulationTime(sim_time.0 - 1),
    );
    let new_activation = f64::max(
        -1.0,
        f64::min(1.0, delta * activation + activation_change_v),
    );

    if agent.activations.len() <= sim_time.0 {
        agent.activations.push(EntityHashMap::default());
    }
    agent.activations[sim_time.0].insert(belief.0, new_activation);
}

fn activation_change(
    agent: &mut Agent,
    actions_at_previous_time: &[(Entity, Entity)],
    belief: &(Entity, &Belief),
    beliefs: &[(Entity, &Belief)],
    sim_time: &SimulationTime,
) -> f64 {
    let pressure = pressure(agent, actions_at_previous_time, belief);
    if pressure > 0.0 {
        (1.0 + contextualise(agent, belief, beliefs, sim_time)) / 2.0 * pressure
    } else {
        (1.0 - contextualise(agent, belief, beliefs, sim_time)) / 2.0 * pressure
    }
}

fn pressure(
    agent: &mut Agent,
    actions_at_previous_time: &[(Entity, Entity)],
    belief: &(Entity, &Belief),
) -> f64 {
    let size = agent.friends.len();
    if size == 0 {
        0.0
    } else {
        actions_at_previous_time
            .iter()
            .map(|(a2, action)| {
                belief.1.perceptions.get(action).unwrap_or(&0.0) * agent.friends.get(a2).unwrap_or(&0.0)
            })
            .sum::<f64>()
            / size as f64
    }
}

fn contextualise(
    agent: &mut Agent,
    belief: &(Entity, &Belief),
    beliefs: &[(Entity, &Belief)],
    sim_time: &SimulationTime,
) -> f64 {
    let size = beliefs.len();
    if size == 0 {
        0.0
    } else {
        beliefs
            .iter()
            .map(|b2| weighted_relationship(agent, belief, b2, sim_time))
            .sum::<f64>()
            / size as f64
    }
}

fn weighted_relationship(
    agent: &mut Agent,
    b1: &(Entity, &Belief),
    b2: &(Entity, &Belief),
    sim_time: &SimulationTime,
) -> f64 {
    let activations = &agent.activations[sim_time.0];
    if activations.contains_key(&b1.0) {
        let relationship = *b1.1.relationships.get(&b2.0).unwrap_or(&0.0);
        activations.get(&b1.0).unwrap_or(&0.0) * relationship
    } else {
        0.0
    }
}
