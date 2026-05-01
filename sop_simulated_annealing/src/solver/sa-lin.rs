use std::time::Instant;

use crate::models::{Problem, Solution};
use crate::rng::EpochRng;
use crate::solver::phase1::tour_improvement;

pub struct StopCriteria {
    pub max_time_secs: u64,
    pub max_evals: u64,
}

pub struct RunStats {
    pub eval_count: u64,
}

#[inline(always)]
fn evaluate_raw(profit: f64, cost: f64, t_max: f64) -> f64 {
    let penalty_factor = 100.0;
    let cost_violation = if cost > t_max { cost - t_max } else { 0.0 };
    profit - (penalty_factor * cost_violation)
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
    let safe_alpha = if alpha <= 0.0 || alpha >= 1.0 {
        0.93
    } else {
        alpha
    };
    let safe_epoch = if epoch_length == 0 {
        80_000
    } else {
        epoch_length
    };

    // Initial polish to start from a good local optimum
    tour_improvement(problem, &mut current_sol);

    let mut best_sol = current_sol.clone();
    let mut best_eval = evaluate_raw(best_sol.total_profit, best_sol.total_cost, problem.t_max);

    let mut temp = t_start;
    let steps = ((t_start - t_final) / (t_start * (1.0 - safe_alpha))).abs().max(1.0);
    let delta_temp = (t_start - t_final) / steps;


    while temp > t_final {
        if should_stop(run_start, stop, stats) {
            break;
        }

        for _ in 0..safe_epoch {
            if should_stop(run_start, stop, stats) {
                break;
            }
            stats.eval_count += 1;

            // Periodically recompute to prevent floating point accumulation drift
            if stats.eval_count % 100_000 == 0 {
                current_sol.recompute(problem);
            }

            let current_eval = evaluate_raw(
                current_sol.total_profit,
                current_sol.total_cost,
                problem.t_max,
            );

            // Expanded Neighborhood Distribution (6 Moves)
            let move_type = rng.next_u64() % 100;

            let mut delta_cost = 0.0;
            let mut delta_profit = 0.0;
            let mut accepted = false;

            let mut apply_insert = None; // (pos, node, cluster)
            let mut apply_replace = None; // (pos, node, cluster)
            let mut apply_drop = None; // (pos)
            let mut apply_swap = None; // (pos1, pos2)
            let mut apply_inversion = None; // (pos1, pos2)
            let mut apply_node_change = None; // (pos, new_node)

            let n_len = current_sol.tour_nodes.len();

            if move_type < 20 && problem.num_clusters > 1 {
                // 1. INSERT (20%)
                let c = (rng.next_u64() as usize % (problem.num_clusters - 1)) + 1;
                if !current_sol.tour_clusters.contains(&c)
                    && !problem.nodes_of_cluster[c].is_empty()
                {
                    let pos = (rng.next_u64() as usize % (n_len - 1)) + 1;
                    let nodes = &problem.nodes_of_cluster[c];
                    let v = nodes[rng.gen_range_usize(0, nodes.len())];

                    let prev = current_sol.tour_nodes[pos - 1];
                    let next = current_sol.tour_nodes[pos];

                    delta_cost = problem.get_dist(prev, v) + problem.get_dist(v, next)
                        - problem.get_dist(prev, next);
                    delta_profit = problem.profits[c];
                    apply_insert = Some((pos, v, c));
                }
            } else if move_type < 40 && n_len > 2 && problem.num_clusters > 1 {
                // 2. REPLACE (20%)
                let pos = (rng.next_u64() as usize % (n_len - 2)) + 1;
                let new_c = (rng.next_u64() as usize % (problem.num_clusters - 1)) + 1;

                if !current_sol.tour_clusters.contains(&new_c)
                    && !problem.nodes_of_cluster[new_c].is_empty()
                {
                    let old_c = current_sol.tour_clusters[pos];
                    let old_v = current_sol.tour_nodes[pos];
                    let nodes = &problem.nodes_of_cluster[new_c];
                    let new_v = nodes[rng.gen_range_usize(0, nodes.len())];

                    let prev = current_sol.tour_nodes[pos - 1];
                    let next = current_sol.tour_nodes[pos + 1];

                    delta_cost = (problem.get_dist(prev, new_v) + problem.get_dist(new_v, next))
                        - (problem.get_dist(prev, old_v) + problem.get_dist(old_v, next));
                    delta_profit = problem.profits[new_c] - problem.profits[old_c];
                    apply_replace = Some((pos, new_v, new_c));
                }
            } else if move_type < 50 && n_len > 3 {
                // 3. DROP (10%)
                let pos = (rng.next_u64() as usize % (n_len - 2)) + 1;
                let old_c = current_sol.tour_clusters[pos];
                let old_v = current_sol.tour_nodes[pos];

                let prev = current_sol.tour_nodes[pos - 1];
                let next = current_sol.tour_nodes[pos + 1];

                delta_cost = problem.get_dist(prev, next)
                    - (problem.get_dist(prev, old_v) + problem.get_dist(old_v, next));
                delta_profit = -problem.profits[old_c];
                apply_drop = Some(pos);
            } else if move_type < 65 && n_len > 3 {
                // 4. SWAP (15%) - From Paper
                let mut i = (rng.next_u64() as usize % (n_len - 2)) + 1;
                let mut j = (rng.next_u64() as usize % (n_len - 2)) + 1;
                if i != j {
                    if i > j {
                        std::mem::swap(&mut i, &mut j);
                    }
                    let prev_i = current_sol.tour_nodes[i - 1];
                    let n_i = current_sol.tour_nodes[i];
                    let next_i = current_sol.tour_nodes[i + 1];
                    let prev_j = current_sol.tour_nodes[j - 1];
                    let n_j = current_sol.tour_nodes[j];
                    let next_j = current_sol.tour_nodes[j + 1];

                    if j == i + 1 {
                        // Adjacent
                        delta_cost = problem.get_dist(prev_i, n_j)
                            + problem.get_dist(n_j, n_i)
                            + problem.get_dist(n_i, next_j)
                            - (problem.get_dist(prev_i, n_i)
                                + problem.get_dist(n_i, n_j)
                                + problem.get_dist(n_j, next_j));
                    } else {
                        // Non-adjacent
                        delta_cost = problem.get_dist(prev_i, n_j)
                            + problem.get_dist(n_j, next_i)
                            + problem.get_dist(prev_j, n_i)
                            + problem.get_dist(n_i, next_j)
                            - (problem.get_dist(prev_i, n_i)
                                + problem.get_dist(n_i, next_i)
                                + problem.get_dist(prev_j, n_j)
                                + problem.get_dist(n_j, next_j));
                    }
                    apply_swap = Some((i, j));
                }
            } else if move_type < 85 && n_len > 4 {
                // 5. INVERSION / 2-OPT (20%) - From Paper
                let i = (rng.next_u64() as usize % (n_len - 3)) + 1;
                let j_range = n_len - 2 - i;
                let j = i
                    + 1
                    + if j_range > 0 {
                        rng.next_u64() as usize % j_range
                    } else {
                        0
                    };

                let prev_i = current_sol.tour_nodes[i - 1];
                let n_i = current_sol.tour_nodes[i];
                let n_j = current_sol.tour_nodes[j];
                let next_j = current_sol.tour_nodes[j + 1];

                // Note: Valid O(1) evaluation because euclidean distance is symmetric in our instance rules
                delta_cost = problem.get_dist(prev_i, n_j) + problem.get_dist(n_i, next_j)
                    - (problem.get_dist(prev_i, n_i) + problem.get_dist(n_j, next_j));
                apply_inversion = Some((i, j));
            } else if n_len > 2 {
                // 6. NODE CHANGE (15%)
                let pos = (rng.next_u64() as usize % (n_len - 2)) + 1;
                let c = current_sol.tour_clusters[pos];
                let nodes = &problem.nodes_of_cluster[c];

                if nodes.len() > 1 {
                    let old_v = current_sol.tour_nodes[pos];
                    let mut new_v = nodes[rng.gen_range_usize(0, nodes.len())];
                    if new_v == old_v {
                        new_v = nodes[(rng.gen_range_usize(1, nodes.len())
                            + nodes.iter().position(|&x| x == old_v).unwrap_or(0))
                            % nodes.len()];
                    }
                    let prev = current_sol.tour_nodes[pos - 1];
                    let next = current_sol.tour_nodes[pos + 1];

                    delta_cost = problem.get_dist(prev, new_v) + problem.get_dist(new_v, next)
                        - (problem.get_dist(prev, old_v) + problem.get_dist(old_v, next));
                    apply_node_change = Some((pos, new_v));
                }
            }

            // Apply Move Logic (Standard Metropolis-Hastings)
            if apply_insert.is_some()
                || apply_replace.is_some()
                || apply_drop.is_some()
                || apply_swap.is_some()
                || apply_inversion.is_some()
                || apply_node_change.is_some()
            {
                let neighbor_profit = current_sol.total_profit + delta_profit;
                let neighbor_cost = current_sol.total_cost + delta_cost;
                let neighbor_eval = evaluate_raw(neighbor_profit, neighbor_cost, problem.t_max);

                let delta = neighbor_eval - current_eval;

                if delta >= 0.0 {
                    accepted = true;
                } else {
                    let prob = f64::exp(delta / temp);
                    if rng.next_f64() < prob {
                        accepted = true;
                    }
                }

                if accepted {
                    if let Some((pos, v, c)) = apply_insert {
                        current_sol.tour_nodes.insert(pos, v);
                        current_sol.tour_clusters.insert(pos, c);
                    } else if let Some((pos, v, c)) = apply_replace {
                        current_sol.tour_nodes[pos] = v;
                        current_sol.tour_clusters[pos] = c;
                    } else if let Some(pos) = apply_drop {
                        current_sol.tour_nodes.remove(pos);
                        current_sol.tour_clusters.remove(pos);
                    } else if let Some((p1, p2)) = apply_swap {
                        current_sol.tour_nodes.swap(p1, p2);
                        current_sol.tour_clusters.swap(p1, p2);
                    } else if let Some((p1, p2)) = apply_inversion {
                        current_sol.tour_nodes[p1..=p2].reverse();
                        current_sol.tour_clusters[p1..=p2].reverse();
                    } else if let Some((pos, v)) = apply_node_change {
                        current_sol.tour_nodes[pos] = v;
                    }

                    current_sol.total_cost = neighbor_cost;
                    current_sol.total_profit = neighbor_profit;

                    // Strictly enforce feasibility for the global Best found
                    if neighbor_cost <= problem.t_max && neighbor_eval > best_eval {
                        best_sol = current_sol.clone();
                        best_eval = neighbor_eval;
                    }
                }
            }
        }
        // temp *= safe_alpha; // Apply cooling
        temp -= delta_temp;
        if temp < t_final {
            temp = t_final;
        }
    }

    tour_improvement(problem, &mut best_sol);
    best_sol
}
