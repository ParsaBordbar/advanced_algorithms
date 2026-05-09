use std::time::Instant;
use crate::models::{Problem, Solution};
use crate::rng::EpochRng;
use crate::solver::phase1_rand::phase1_randomized_construction;
use crate::solver::phase1_construction::tour_improvement; 

pub fn grasp(
    pb: &Problem, 
    rng: &mut EpochRng, 
    alpha: f64,
    max_time_secs: u64,
    max_evals: u64,
    start_time: Instant,
) -> (Solution, u64) {
    let mut best_sol = Solution::new();
    let mut eval_count: u64 = 0;
    let mut iter_without_improvement = 0;
    
    // Convergence limit when no specific time/eval limits are provided
    let max_convergence_iter = 5000; 
    
    loop {
        // --- STOPPING CRITERIA CHECKS ---
        if max_time_secs > 0 && start_time.elapsed().as_secs() >= max_time_secs {
            println!("Stop Reason: Time limit reached.");
            break;
        }
        
        if max_evals > 0 && eval_count >= max_evals {
            println!("Stop Reason: Evaluation limit reached.");
            break;
        }

        // Check for convergence (ONLY if no other limits are set)
        if max_time_secs == 0 && max_evals == 0 && iter_without_improvement >= max_convergence_iter {
            println!("Stop Reason: Convergence limit reached.");
            break;
        }

        // 1. Construct a randomized greedy solution
        let mut current_sol = phase1_randomized_construction(pb, rng, alpha);
        
        // 2. Apply Local Search to reach local optimum
        tour_improvement(pb, &mut current_sol);
        
        // We count one full GRASP iteration (Construction + Local Search) as one evaluation block.
        // If you passed eval_count into tour_improvement or phase1_randomized_construction, 
        // you would update it there instead.
        eval_count += 1; 
        
        // 3. Update best found solution
        let global_better_profit = current_sol.total_profit > best_sol.total_profit + 1e-5;
        let global_tie_breaker = (current_sol.total_profit - best_sol.total_profit).abs() < 1e-5 
                                  && current_sol.total_cost < best_sol.total_cost;

        if global_better_profit || global_tie_breaker {
            best_sol = current_sol;
            iter_without_improvement = 0; // Reset convergence counter
        } else {
            iter_without_improvement += 1;
        }
    }
    
    (best_sol, eval_count)
}
