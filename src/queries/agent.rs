use crate::components::agent::Agent;
use crate::components::behaviour::Behaviour;
use crate::components::belief::Belief;
use crate::resources::time::SimulationTime;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy_rand::global::GlobalRng;
use bevy_rand::prelude::WyRand;
use itertools::Itertools;
use rand::prelude::*;

pub fn perform_actions(
    mut agent_query: Query<&mut Agent>,
    belief_query: Query<Entity, With<Belief>>,
    behaviour_query: Query<Entity, With<Behaviour>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    sim_time: Res<SimulationTime>,
) {
    info!("[time={}] performing actions", sim_time.0);
    for mut agent in agent_query.iter_mut() {
        perform_action(&mut agent, belief_query, behaviour_query, &mut rng);
    }
}

fn perform_action(
    agent: &mut Agent,
    belief_query: Query<Entity, With<Belief>>,
    behaviour_query: Query<Entity, With<Behaviour>>,
    rng: &mut WyRand,
) {
    let unnormalized_probabilities: Vec<(Entity, f64)> = behaviour_query
        .iter()
        .map(|behaviour_entity| {
            (
                behaviour_entity,
                belief_query
                    .iter()
                    .map(|belief_entity| {
                        agent
                            .performance_relationships
                            .get(&belief_entity)
                            .expect("Missing performance relationship")
                            .get(&behaviour_entity)
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
            .into_iter()
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
            let normalized_probabilities: Vec<(Entity, f64)> = filtered_probs
                .into_iter()
                .map(|(entity, prob)| (entity, prob / normalizing_factor))
                .collect();
            let selected_action = normalized_probabilities
                .choose_weighted(rng, |(_, prob)| *prob)
                .unwrap();
            agent.actions.push(selected_action.0)
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

    for (_, mut agent) in agent_query.iter_mut() {
        update_activations_for_all_belief(&mut agent, &actions, &belief_query, &sim_time);
    }
}

fn update_activations_for_all_belief(
    agent: &mut Agent,
    actions_at_previous_time: &Vec<(Entity, Entity)>,
    belief_query: &Query<(Entity, &Belief)>,
    sim_time: &Res<SimulationTime>,
) {
    for entry in belief_query.iter() {
        update_activation(
            agent,
            actions_at_previous_time,
            &entry,
            belief_query,
            &sim_time,
        );
    }
}

fn update_activation(
    agent: &mut Agent,
    actions_at_previous_time: &Vec<(Entity, Entity)>,
    belief: &(Entity, &Belief),
    belief_query: &Query<(Entity, &Belief)>,
    sim_time: &SimulationTime,
) {
    let delta = *agent.deltas.get(&belief.0).expect("Missing delta");
    let activations = &agent.activations[sim_time.0 - 1];
    let activation = *activations.get(&belief.0).unwrap_or(&0.0);

    let activation_change_v = activation_change(
        agent,
        actions_at_previous_time,
        belief,
        belief_query,
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
    actions_at_previous_time: &Vec<(Entity, Entity)>,
    belief: &(Entity, &Belief),
    belief_query: &Query<(Entity, &Belief)>,
    sim_time: &SimulationTime,
) -> f64 {
    let pressure = pressure(agent, actions_at_previous_time, belief);
    if pressure > 0.0 {
        (1.0 + contextualise(agent, belief, belief_query, sim_time)) / 2.0 * pressure
    } else {
        (1.0 - contextualise(agent, belief, belief_query, sim_time)) / 2.0 * pressure
    }
}

fn pressure(
    agent: &mut Agent,
    actions_at_previous_time: &Vec<(Entity, Entity)>,
    belief: &(Entity, &Belief),
) -> f64 {
    let size = agent.friends.len();
    if size == 0 {
        0.0
    } else {
        actions_at_previous_time
            .as_slice()
            .iter()
            .map(|(a2, action)| {
                belief.1.perceptions.get(action).unwrap_or(&0.0) * agent.friends.get(a2).unwrap()
            })
            .sum::<f64>()
            / size as f64
    }
}

fn contextualise(
    agent: &mut Agent,
    belief: &(Entity, &Belief),
    belief_query: &Query<(Entity, &Belief)>,
    sim_time: &SimulationTime,
) -> f64 {
    let size = belief_query.iter().len();
    if size == 0 {
        0.0
    } else {
        belief_query
            .iter()
            .map(|b2| weighted_relationship(agent, belief, &b2, sim_time))
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
