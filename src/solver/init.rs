use crate::models::{Problem, Solution};
use crate::rng::EpochRng;

pub fn random_feasible_initial_solution(problem: &Problem, rng: &mut EpochRng) -> Solution {
    let mut tour_nodes = vec![0, 0];
    let mut tour_clusters = vec![0, 0];
    let mut total_cost = 0.0;
    let mut total_profit = 0.0;

    // Get all valid non-depot clusters and shuffle them to randomize the path
    let mut cluster_ids: Vec<usize> = (1..problem.num_clusters).collect();
    rng.shuffle(&mut cluster_ids);

    // Incrementally attempt to add clusters
    for &c in &cluster_ids {
        let nodes = &problem.nodes_of_cluster[c];
        if nodes.is_empty() {
            continue;
        }

        // Pick a completely random node from the cluster
        let random_node_idx = rng.gen_range_usize(0, nodes.len());
        let candidate_node = nodes[random_node_idx];

        let last_visited = tour_nodes[tour_nodes.len() - 2];

        let cost_removed = problem.get_dist(last_visited, 0);
        let cost_added =
            problem.get_dist(last_visited, candidate_node) + problem.get_dist(candidate_node, 0);
        let delta_cost = cost_added - cost_removed;

        if total_cost + delta_cost <= problem.t_max {
            // Apply the move
            tour_nodes.insert(tour_nodes.len() - 1, candidate_node);
            tour_clusters.insert(tour_clusters.len() - 1, c);

            total_cost += delta_cost;
            total_profit += problem.profits[c];
        }
    }

    Solution {
        tour_nodes,
        tour_clusters,
        total_profit,
        total_cost,
    }
}
