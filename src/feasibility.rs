use std::collections::HashSet;

use crate::models::{Problem, Solution};

/// Returns true if solution satisfies SOP constraints.
/// Checks:
/// 1) Route starts/ends at depot node 0
/// 2) tour_nodes and tour_clusters lengths match
/// 3) Each node belongs to its declared cluster
/// 4) No duplicated non-depot cluster in route
/// 5) Total travel time <= Tmax
pub fn is_feasible(problem: &Problem, sol: &Solution) -> bool {
    if sol.tour_nodes.len() < 2 || sol.tour_clusters.len() < 2 {
        return false;
    }

    // Must start and end at depot
    if sol.tour_nodes.first() != Some(&0) || sol.tour_nodes.last() != Some(&0) {
        return false;
    }

    if sol.tour_clusters.first() != Some(&0) || sol.tour_clusters.last() != Some(&0) {
        return false;
    }

    if sol.tour_nodes.len() != sol.tour_clusters.len() {
        return false;
    }

    // Node-cluster consistency
    for (&node, &cluster) in sol.tour_nodes.iter().zip(sol.tour_clusters.iter()) {
        if node >= problem.num_nodes || cluster >= problem.num_clusters {
            return false;
        }
        if problem.cluster_of_node[node] != cluster {
            return false;
        }
    }

    // No duplicate visited customer clusters (exclude depot positions)
    let mut seen_clusters = HashSet::new();
    if sol.tour_clusters.len() > 2 {
        for &c in &sol.tour_clusters[1..sol.tour_clusters.len() - 1] {
            if c == 0 {
                return false; // depot in middle not allowed in this representation
            }
            if !seen_clusters.insert(c) {
                return false;
            }
        }
    }

    // Time feasibility
    let mut cost = 0.0;
    for i in 0..sol.tour_nodes.len() - 1 {
        let a = sol.tour_nodes[i];
        let b = sol.tour_nodes[i + 1];
        cost += problem.get_dist(a, b);
    }

    cost <= problem.t_max + 1e-9
}
