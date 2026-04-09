use crate::models::{Problem, Solution};
use crate::rng::EpochRng;

#[inline(always)]
fn evaluate(sol: &Solution, problem: &Problem) -> f64 {
    let penalty_factor = 10.0;
    let cost_violation = if sol.total_cost > problem.t_max {
        sol.total_cost - problem.t_max
    } else {
        0.0
    };
    sol.total_profit as f64 - (penalty_factor * cost_violation)
}

/// Simulated Annealing algorithm
pub fn simulated_annealing(
    problem: &Problem,
    mut current_sol: Solution,
    t_start: f64,
    t_final: f64,
    alpha: f64,
    epoch_length: usize,
    rng: &mut EpochRng,
) -> Solution {
    // Safety check to prevent infinite loops!
    let safe_alpha = if alpha >= 1.0 { 0.95 } else { alpha };

    let mut best_sol = current_sol.clone();
    let mut current_eval = evaluate(&current_sol, problem);
    let mut best_eval = current_eval;

    let mut temp = t_start;

    // Outer loop: Cooling schedule
    while temp > t_final {
        for _ in 0..epoch_length {
            let is_insert = rng.next_u64() % 2 == 0;
            let mut neighbor_sol = current_sol.clone();

            let moved = if is_insert {
                // Random Insert
                if neighbor_sol.tour_clusters.len() <= problem.num_clusters {
                    // Fix: Generate customer clusters only (1 to num_clusters - 2)
                    let c = (rng.next_u64() as usize % (problem.num_clusters - 2)) + 1;
                    
                    // We can insert anywhere except index 0 (must stay start node) and the very end
                    let max_pos = std::cmp::max(1, neighbor_sol.tour_clusters.len() - 1);
                    let pos = (rng.next_u64() as usize % max_pos) + 1; 

                    if !neighbor_sol.tour_clusters.contains(&c) {
                        neighbor_sol.tour_clusters.insert(pos, c);
                        true
                    } else { false }
                } else { false }
            } else {
                // Random Swap
                if neighbor_sol.tour_clusters.len() > 2 {
                    // Only swap internal nodes (avoid index 0 and the last index)
                    let pos = (rng.next_u64() as usize % (neighbor_sol.tour_clusters.len() - 2)) + 1;
                    
                    // Fix: Generate customer clusters only (1 to num_clusters - 2)
                    let c = (rng.next_u64() as usize % (problem.num_clusters - 2)) + 1;
                    
                    if !neighbor_sol.tour_clusters.contains(&c) {
                        neighbor_sol.tour_clusters[pos] = c;
                        true
                    } else { false }
                } else { false }
            };

            if !moved {
                continue; 
            }

            neighbor_sol.update_nodes_greedy(problem);

            neighbor_sol.recompute(problem);
            let neighbor_eval = evaluate(&neighbor_sol, problem);

            let delta = neighbor_eval - current_eval;

            let accept = if delta > 0.0 {
                true
            } else {
                let prob = 1.0 / (1.0 + f64::exp(-delta / temp));
                rng.next_f64() < prob
            };

            if accept {
                current_sol = neighbor_sol;
                current_eval = neighbor_eval;

                if current_eval > best_eval && current_sol.total_cost <= problem.t_max {
                    best_sol = current_sol.clone();
                    best_eval = current_eval;
                }
            }
        }

        temp *= safe_alpha;
    }

    best_sol
}
