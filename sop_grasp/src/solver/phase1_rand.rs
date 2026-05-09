use crate::models::{Problem, Solution};
use crate::rng::EpochRng;

pub fn phase1_randomized_construction(pb: &Problem, rng: &mut EpochRng, alpha: f64) -> Solution {
    let mut sol = Solution::new();
    let mut unvisited: Vec<usize> = (1..pb.num_clusters).collect();
    
    loop {
        let mut candidates = Vec::new();
        let mut g_max = f64::MIN;
        let mut g_min = f64::MAX;

        let last_node = *sol.tour_nodes.iter().rev().nth(1).unwrap_or(&0);

        for &c in &unvisited {
            let profit = pb.profits[c]; // Already f64 based on your models
            
            // Find the best node in cluster c to minimize distance from last_node and to depot
            let mut best_node = 0;
            let mut best_cost_add = f64::MAX; // FIX: Changed from i32::MAX to f64::MAX

            for &v in &pb.nodes_of_cluster[c] {
                // pb.get_dist returns f64, so cost_add is f64
                let cost_add = pb.get_dist(last_node, v) + pb.get_dist(v, 0) - pb.get_dist(last_node, 0);
                if cost_add < best_cost_add {
                    best_cost_add = cost_add;
                    best_node = v;
                }
            }

            if sol.total_cost + best_cost_add <= pb.t_max {
                let heuristic_value = profit / (best_cost_add + 1e-6); // Avoid div by zero
                candidates.push((c, best_node, best_cost_add, heuristic_value));
                if heuristic_value > g_max { g_max = heuristic_value; }
                if heuristic_value < g_min { g_min = heuristic_value; }
            }
        }

        if candidates.is_empty() {
            break; // No more feasible clusters to add
        }

        // Build RCL (Restricted Candidate List)
        let threshold = g_max - alpha * (g_max - g_min);
        let rcl: Vec<_> = candidates.into_iter().filter(|&(_, _, _, g)| g >= threshold).collect();

        // Pick uniformly at random from RCL
        let pick_idx = (rng.next_u32() as usize) % rcl.len();
        let (chosen_cluster, chosen_node, cost_add, _) = rcl[pick_idx];

        // Insert into solution right before the end depot
        let end_idx = sol.tour_nodes.len() - 1;
        sol.tour_nodes.insert(end_idx, chosen_node);
        sol.tour_clusters.insert(end_idx, chosen_cluster);
        
        sol.total_cost += cost_add;
        sol.total_profit += pb.profits[chosen_cluster];

        unvisited.retain(|&x| x != chosen_cluster);
    }

    sol
}
