use std::collections::HashSet;
use std::time::Instant;
use crate::models::{Problem, Solution};
use crate::rng::XorShift;
use crate::solver::phase1::tour_improvement;

pub fn tabu_search(
    pb: &Problem, 
    mut current_sol: Solution, 
    lambda: f64, 
    beta: f64, 
    alpha: usize, 
    max_time_secs: u64,
    max_evals: u64,
    start_time: Instant,
) -> (Solution, u64) {
    let mut rng = XorShift::new(42);
    let l_lambda = (pb.num_clusters as f64 * lambda).ceil() as u64;

    let mut best_sol = current_sol.clone();
    
    let mut tabu_insert = vec![0usize; pb.num_clusters];
    let mut tabu_remove = vec![0usize; pb.num_clusters];

    let mut iter_without_improvement = 0;
    let mut total_iters = 0;
    let mut eval_count: u64 = 0;

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

        total_iters += 1;

        // Pass eval_count mutably to track objective evaluations
        let neighbor = explore_neighborhood(pb, &current_sol, &tabu_insert, &tabu_remove, total_iters, &mut eval_count);

        let next_sol;
        let mut used_move_cluster = 0;
        let mut is_insertion = true;

        if let Some((cand_sol, cluster_changed, is_ins)) = neighbor {
            next_sol = Some(cand_sol);
            used_move_cluster = cluster_changed;
            is_insertion = is_ins;
        } else {
            // Apply exact MASOP SoftShake/HardShake 
            let shaken_sol = apply_shake(&current_sol, &best_sol, beta, &mut rng, pb);
            next_sol = Some(shaken_sol);
        }

        if let Some(new_sol) = next_sol {
            current_sol = new_sol.clone();

            if used_move_cluster != 0 {
                let tenure = rng.rand_tenure(l_lambda);
                if is_insertion {
                    tabu_remove[used_move_cluster] = total_iters + tenure;
                } else {
                    tabu_insert[used_move_cluster] = total_iters + tenure;
                }
            }

            if total_iters % alpha == 0 {
                tour_improvement(pb, &mut current_sol);
                eval_count += 1; // Count local search improvement as an evaluation
            }

            let global_better_profit = current_sol.total_profit > best_sol.total_profit + 1e-5;
            let global_tie_breaker = (current_sol.total_profit - best_sol.total_profit).abs() < 1e-5 
                                      && current_sol.total_cost < best_sol.total_cost;

            if global_better_profit || global_tie_breaker {
                tour_improvement(pb, &mut current_sol);
                eval_count += 1; // Count local search improvement as an evaluation
                best_sol = current_sol.clone();
                iter_without_improvement = 0;
            } else {
                iter_without_improvement += 1;
            }
        } else {
            iter_without_improvement += 1;
        }
    }
    
    (best_sol, eval_count)
}

/// Diversification phase: SoftShake (5-15%) and HardShake (30-40%)
fn apply_shake(
    current: &Solution, 
    best: &Solution, 
    beta: f64, 
    rng: &mut XorShift, 
    pb: &Problem
) -> Solution {
    let mut shaken_sol = current.clone();
    let len = shaken_sol.tour_clusters.len();
    
    let n_visited = len.saturating_sub(2);
    if n_visited <= 1 {
        return shaken_sol;
    }

    let threshold = best.total_profit - (beta * best.total_profit);
    
    let (min_pct, max_pct) = if current.total_profit >= threshold {
        (0.05, 0.15) 
    } else {
        (0.30, 0.40) 
    };

    let mut min_k = (n_visited as f64 * min_pct).ceil() as usize;
    let mut max_k = (n_visited as f64 * max_pct).ceil() as usize;
    
    if min_k == 0 { min_k = 1; }
    if max_k > n_visited { max_k = n_visited; }
    if min_k > max_k { min_k = max_k; }

    let k = if min_k < max_k {
        min_k + (rng.next() as usize % (max_k - min_k + 1))
    } else {
        min_k
    };

    let mut indices: Vec<usize> = (1..len - 1).collect();
    for i in 0..k {
        let swap_idx = i + (rng.next() as usize % (indices.len() - i));
        indices.swap(i, swap_idx);
    }

    let mut keep = vec![true; len];
    for i in 0..k {
        keep[indices[i]] = false;
    }

    let mut idx = 0;
    shaken_sol.tour_clusters.retain(|_| {
        let keep_it = keep[idx];
        idx += 1;
        keep_it
    });
    
    let mut idx2 = 0;
    shaken_sol.tour_nodes.retain(|_| {
        let keep_it = keep[idx2];
        idx2 += 1;
        keep_it
    });

    shaken_sol.recompute(pb);
    shaken_sol
}

pub fn explore_neighborhood(
    pb: &Problem,
    current: &Solution,
    tabu_insert: &[usize],
    tabu_remove: &[usize],
    current_iter: usize,
    eval_count: &mut u64, // Added parameter
) -> Option<(Solution, usize, bool)> {
    
    let mut best_neighbor: Option<Solution> = None;
    let mut best_move_cluster = 0;
    let mut best_is_insertion = true;

    let visited_set: HashSet<usize> = current.tour_clusters.iter().cloned().collect();
    
    let evaluate = |cand: &Solution, best: &Option<Solution>| -> bool {
        if let Some(b) = best {
            if cand.total_profit > b.total_profit + 1e-5 { return true; }
            if (cand.total_profit - b.total_profit).abs() < 1e-5 && cand.total_cost < b.total_cost { return true; }
            return false;
        }
        true
    };

    // 1. Evaluate Insert Moves
    for cg in 1..pb.num_clusters {
        if visited_set.contains(&cg) { continue; }
        let is_tabu = tabu_insert[cg] > current_iter;

        for pos in 1..current.tour_nodes.len() {
            let u = current.tour_nodes[pos - 1];
            let w = current.tour_nodes[pos];

            let mut best_v = 0;
            let mut min_delta = f64::MAX;

            for &v in &pb.nodes_of_cluster[cg] {
                // Evaluating node cost
                *eval_count += 1; 
                let delta = pb.get_dist(u, v) + pb.get_dist(v, w) - pb.get_dist(u, w);
                if delta < min_delta {
                    min_delta = delta;
                    best_v = v;
                }
            }

            if current.total_cost + min_delta <= pb.t_max {
                let mut cand = current.clone();
                cand.tour_nodes.insert(pos, best_v);
                cand.tour_clusters.insert(pos, cg);
                cand.total_cost += min_delta;
                cand.total_profit += pb.profits[cg];

                if !is_tabu {
                    if evaluate(&cand, &best_neighbor) {
                        best_neighbor = Some(cand);
                        best_move_cluster = cg;
                        best_is_insertion = true;
                    }
                }
            }
        }
    }

    // 2. Evaluate Swap Moves
    for cg in 1..pb.num_clusters {
        if visited_set.contains(&cg) { continue; }

        for pos_out in 1..(current.tour_nodes.len() - 1) {
            let ch = current.tour_clusters[pos_out];
            let is_tabu = tabu_insert[cg] > current_iter || tabu_remove[ch] > current_iter;
            
            let mut inter_sol = current.clone();
            let u_rem = inter_sol.tour_nodes[pos_out - 1];
            let w_rem = inter_sol.tour_nodes[pos_out + 1];
            let v_rem = inter_sol.tour_nodes[pos_out];
            
            inter_sol.total_cost -= pb.get_dist(u_rem, v_rem) + pb.get_dist(v_rem, w_rem) - pb.get_dist(u_rem, w_rem);
            inter_sol.total_profit -= pb.profits[ch];
            inter_sol.tour_nodes.remove(pos_out);
            inter_sol.tour_clusters.remove(pos_out);

            for pos_in in 1..inter_sol.tour_nodes.len() {
                let u = inter_sol.tour_nodes[pos_in - 1];
                let w = inter_sol.tour_nodes[pos_in];

                let mut best_v = 0;
                let mut min_delta = f64::MAX;

                for &v in &pb.nodes_of_cluster[cg] {
                    // Evaluating node cost
                    *eval_count += 1;
                    let delta = pb.get_dist(u, v) + pb.get_dist(v, w) - pb.get_dist(u, w);
                    if delta < min_delta {
                        min_delta = delta;
                        best_v = v;
                    }
                }

                if inter_sol.total_cost + min_delta <= pb.t_max {
                    let mut cand = inter_sol.clone();
                    cand.tour_nodes.insert(pos_in, best_v);
                    cand.tour_clusters.insert(pos_in, cg);
                    cand.total_cost += min_delta;
                    cand.total_profit += pb.profits[cg];

                    if !is_tabu {
                        if evaluate(&cand, &best_neighbor) {
                            best_neighbor = Some(cand);
                            best_move_cluster = cg; 
                            best_is_insertion = true; 
                        }
                    }
                }
            }
        }
    }

    best_neighbor.map(|sol| (sol, best_move_cluster, best_is_insertion))
}
