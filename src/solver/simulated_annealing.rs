use std::time::Instant;

use crate::models::{Problem, Solution};
use crate::rng::EpochRng;

pub struct StopCriteria {
    pub max_time_secs: u64, // 0 => unlimited
    pub max_evals: u64,     // 0 => unlimited
}

pub struct RunStats {
    pub eval_count: u64,
}

#[inline(always)]
fn evaluate(sol: &Solution, problem: &Problem, stats: &mut RunStats) -> f64 {
    stats.eval_count += 1;

    let penalty_factor = 10.0;
    let cost_violation = if sol.total_cost > problem.t_max {
        sol.total_cost - problem.t_max
    } else {
        0.0
    };

    sol.total_profit - (penalty_factor * cost_violation)
}

#[inline(always)]
fn should_stop(start: Instant, stop: &StopCriteria, stats: &RunStats) -> bool {
    let time_stop = stop.max_time_secs > 0 && start.elapsed().as_secs() >= stop.max_time_secs;
    let eval_stop = stop.max_evals > 0 && stats.eval_count >= stop.max_evals;
    time_stop || eval_stop
}

pub fn simulated_annealing(
    problem: &Problem,
    mut current_sol: Solution,
    t_start: f64,
    t_final: f64,
    alpha: f64,
    epoch_length: usize,
    rng: &mut EpochRng,
    stop: &StopCriteria,
    stats: &mut RunStats,
) -> Solution {
    let run_start = Instant::now();

    // protect from infinite loop
    let safe_alpha = if alpha >= 1.0 { 0.95 } else { alpha };

    let mut best_sol = current_sol.clone();
    let mut current_eval = evaluate(&current_sol, problem, stats);
    let mut best_eval = current_eval;

    let mut temp = t_start;

    while temp > t_final {
        if should_stop(run_start, stop, stats) {
            break;
        }

        for _ in 0..epoch_length {
            if should_stop(run_start, stop, stats) {
                break;
            }

            let is_insert = rng.next_u64() % 2 == 0;
            let mut neighbor_sol = current_sol.clone();

            let moved = if is_insert {
                // Random Insert
                if neighbor_sol.tour_clusters.len() <= problem.num_clusters {
                    // valid customer cluster range: [1, num_clusters-1]
                    if problem.num_clusters <= 1 {
                        false
                    } else {
                        let c = (rng.next_u64() as usize % (problem.num_clusters - 1)) + 1;

                        // insert position in internal segment [1 .. len-1]
                        let max_pos = std::cmp::max(1, neighbor_sol.tour_clusters.len() - 1);
                        let pos = (rng.next_u64() as usize % max_pos) + 1;

                        if !neighbor_sol.tour_clusters.contains(&c) {
                            neighbor_sol.tour_clusters.insert(pos, c);
                            true
                        } else {
                            false
                        }
                    }
                } else {
                    false
                }
            } else {
                // Random Replace (called swap before)
                if neighbor_sol.tour_clusters.len() > 2 && problem.num_clusters > 1 {
                    let pos = (rng.next_u64() as usize % (neighbor_sol.tour_clusters.len() - 2)) + 1;
                    let c = (rng.next_u64() as usize % (problem.num_clusters - 1)) + 1;

                    if !neighbor_sol.tour_clusters.contains(&c) {
                        neighbor_sol.tour_clusters[pos] = c;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if !moved {
                continue;
            }

            neighbor_sol.update_nodes_greedy(problem);
            neighbor_sol.recompute(problem);

            let neighbor_eval = evaluate(&neighbor_sol, problem, stats);
            let delta = neighbor_eval - current_eval;

            let accept = if delta >= 0.0 {
                true
            } else {
                let prob = f64::exp(delta / temp);
                rng.next_f64() < prob
            };

            if accept {
                current_sol = neighbor_sol;
                current_eval = neighbor_eval;

                // keep best feasible solution
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
