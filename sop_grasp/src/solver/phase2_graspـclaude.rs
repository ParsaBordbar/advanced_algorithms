// =============================================================================
// GRASP Solver for the Set Orienteering Problem (SOP)
//
// DESIGN OVERVIEW:
// ─────────────────────────────────────────────────────────────────────────────
// GRASP (Greedy Randomized Adaptive Search Procedure) is a multi-start
// metaheuristic with two phases per iteration:
//
//   1. CONSTRUCTION: Build a feasible solution using a randomised greedy
//      strategy. We compute a Restricted Candidate List (RCL) of the best
//      insertable clusters at each step, then pick one uniformly at random.
//      This creates diverse, good-quality starting points.
//
//   2. LOCAL SEARCH: Improve the constructed solution via neighbourhood moves
//      until no further improvement is possible (local optimum).
//
// The global best across ALL iterations is returned.
//
// WHY GRASP CAN BEAT PURE TABU SEARCH:
//   - Tabu intensifies around one basin of attraction; GRASP explores many.
//   - Each restart is independent → immune to being "trapped."
//   - Combining GRASP construction with strong local search gives both breadth
//     (exploration) and depth (exploitation).
//
// NEIGHBOURHOODS USED IN LOCAL SEARCH:
//   A. DP Vertex Selection  – for each visited cluster, pick the cheapest node
//      (dynamic programming over the sequence). Inherited from phase1.rs.
//   B. 2-Opt Routing        – reverse sub-sequences to cut crossing edges.
//      Inherited from phase1.rs.
//   C. Or-Opt (Relocate)    – move a single visited cluster to every other
//      position in the tour. Strictly stronger than 2-opt for this problem
//      because cluster order matters independently of node selection.
//   D. Remove-and-Reinsert  – drop the cluster with the worst
//      profit-per-unit-cost ratio, then try inserting every unvisited cluster
//      at its cheapest position. Escapes "wrong cluster" local optima.
//   E. Swap                 – exchange one visited cluster for one unvisited
//      cluster when the swap is immediately profitable.
//
// ADAPTIVE α (RCL CONTROL):
//   α ∈ [0, 1] controls how greedy vs. random the construction is.
//   α = 0 → pure greedy (always best candidate).
//   α = 1 → pure random.
//   We start at α_start and widen (increase) α when recent iterations have
//   produced no improvement. This is "reactive GRASP."
//
// ELITE POOL:
//   We keep the top-K distinct solutions found so far. Periodically we restart
//   the construction from a perturbed elite solution rather than from scratch.
//   This is called "GRASP with Path Relinking" (simplified variant).
//
// STOPPING CRITERIA (same interface as your Tabu Search):
//   - Wall-clock time limit (max_time_secs > 0)
//   - Evaluation budget    (max_evals > 0)
//   - Convergence limit    (when both above are 0)
// =============================================================================

use std::collections::HashSet;
use std::time::Instant;

use crate::models::{Problem, Solution};
use crate::rng::EpochRng;
use crate::solver::phase1::{optimize_routing_2opt, optimize_vertices_dp, tour_improvement};

// ─────────────────────────────────────────────────────────────────────────────
// PUBLIC ENTRY POINT
// ─────────────────────────────────────────────────────────────────────────────

/// Run GRASP for SOP. Returns `(best_solution, eval_count)`.
///
/// Parameters
/// ──────────
/// `alpha_start`     Initial RCL greediness [0..1]. 0.15 is a good default.
/// `elite_size`      How many elite solutions to keep. 5 is typical.
/// `max_time_secs`   Hard wall-clock limit in seconds (0 = disabled).
/// `max_evals`       Hard evaluation-count limit (0 = disabled).
/// `start_time`      Instant::now() captured before parsing (shared timer).
pub fn grasp(
    pb: &Problem,
    alpha_start: f64,
    elite_size: usize,
    max_time_secs: u64,
    max_evals: u64,
    start_time: Instant,
) -> (Solution, u64) {
    let mut rng = EpochRng::new();
    let mut eval_count: u64 = 0;
    let mut best_sol = Solution::new(); // empty tour, zero profit
    let mut elite: Vec<Solution> = Vec::with_capacity(elite_size + 1);

    // Adaptive α tracking
    let mut alpha = alpha_start;
    let alpha_max: f64 = 0.60; // never go fully random
    let mut iters_no_improve: usize = 0;
    let adapt_after: usize = 8; // widen α every N non-improving iters
    let adapt_step: f64 = 0.05;

    // Convergence fallback when no explicit limit is given
    let convergence_limit: usize = 300;
    let mut total_iters: usize = 0;

    loop {
        // ── STOPPING CRITERIA ──────────────────────────────────────────────
        if max_time_secs > 0 && start_time.elapsed().as_secs() >= max_time_secs {
            println!("GRASP Stop: Time limit reached ({} s).", max_time_secs);
            break;
        }
        if max_evals > 0 && eval_count >= max_evals {
            println!("GRASP Stop: Evaluation budget exhausted ({} evals).", max_evals);
            break;
        }
        if max_time_secs == 0 && max_evals == 0 && iters_no_improve >= convergence_limit {
            println!("GRASP Stop: Convergence (no improvement for {} iters).", convergence_limit);
            break;
        }

        total_iters += 1;

        // ── PHASE 1: CONSTRUCTION ──────────────────────────────────────────
        // Occasionally seed from an elite solution (perturbation restart).
        // Every 7th iteration after we have enough elites, we destroy part of
        // an elite solution and rebuild — this is a lightweight path-relinking.
        let constructed = if elite.len() >= 3 && total_iters % 7 == 0 {
            let elite_idx = rng.gen_range_usize(0, elite.len());
            let seed = elite[elite_idx].clone();
            perturbation_restart(pb, seed, alpha, &mut rng, &mut eval_count)
        } else {
            grasp_construct(pb, alpha, &mut rng, &mut eval_count)
        };

        // ── PHASE 2: LOCAL SEARCH ──────────────────────────────────────────
        let improved = full_local_search(pb, constructed, &mut eval_count);

        // ── UPDATE BEST & ELITE ────────────────────────────────────────────
        let better_profit = improved.total_profit > best_sol.total_profit + 1e-5;
        let tie_cheaper = (improved.total_profit - best_sol.total_profit).abs() < 1e-5
            && improved.total_cost < best_sol.total_cost - 1e-5;

        if better_profit || tie_cheaper {
            best_sol = improved.clone();
            iters_no_improve = 0;
            // Reset α toward greedy when we improve (we found a good region)
            alpha = (alpha - adapt_step).max(alpha_start);
            println!(
                "  GRASP iter {:4} | new best profit={:.0}  cost={:.0}  evals={}",
                total_iters, best_sol.total_profit, best_sol.total_cost, eval_count
            );
        } else {
            iters_no_improve += 1;
        }

        update_elite(&mut elite, improved, elite_size);

        // ── ADAPTIVE α ────────────────────────────────────────────────────
        // If we are stuck, diversify by widening the RCL.
        if iters_no_improve > 0 && iters_no_improve % adapt_after == 0 {
            alpha = (alpha + adapt_step).min(alpha_max);
        }
    }

    println!(
        "GRASP finished: {} iterations, {} evaluations, best profit={}",
        total_iters, eval_count, best_sol.total_profit
    );

    (best_sol, eval_count)
}

// ─────────────────────────────────────────────────────────────────────────────
// CONSTRUCTION PHASE
// ─────────────────────────────────────────────────────────────────────────────

/// Build one feasible solution using the randomised greedy (GRASP) strategy.
///
/// Algorithm:
///   While there are unvisited clusters and some can still be inserted:
///     1. For each unvisited cluster cg, find the cheapest insertion position
///        and the cheapest node within cg at that position.  Record
///        (insertion_cost, profit, cg, node, position).
///     2. Compute a "score" = profit / insertion_cost  (ratio heuristic).
///        When insertion_cost ≈ 0, the cluster is essentially free — score = ∞.
///     3. score_max = max score among all candidates.
///        score_min = min score among all candidates.
///        RCL threshold = score_max − α × (score_max − score_min).
///        A candidate enters the RCL if its score ≥ threshold.
///     4. Pick one entry from the RCL uniformly at random and apply the move.
///
/// α = 0 → always pick the single best (pure greedy, deterministic).
/// α = 1 → the whole candidate list qualifies (pure random).
fn grasp_construct(
    pb: &Problem,
    alpha: f64,
    rng: &mut EpochRng,
    eval_count: &mut u64,
) -> Solution {
    let mut sol = Solution::new(); // starts as depot → depot

    // Track which clusters are already in the tour
    let mut in_tour: Vec<bool> = vec![false; pb.num_clusters];
    in_tour[0] = true; // depot cluster is always "visited"

    loop {
        // Collect all feasible insertion candidates
        // A candidate = (score, cluster_id, best_node, best_position)
        let mut candidates: Vec<(f64, usize, usize, usize)> = Vec::new();

        for cg in 1..pb.num_clusters {
            if in_tour[cg] {
                continue;
            }

            // Find cheapest insertion: try every gap in the current tour
            // Gap i is between tour_nodes[i-1] and tour_nodes[i].
            let mut best_delta = f64::MAX;
            let mut best_node = 0;
            let mut best_pos = 0;

            for pos in 1..sol.tour_nodes.len() {
                let u = sol.tour_nodes[pos - 1];
                let w = sol.tour_nodes[pos];
                let base_edge = pb.get_dist(u, w);

                for &v in &pb.nodes_of_cluster[cg] {
                    *eval_count += 1;
                    let delta = pb.get_dist(u, v) + pb.get_dist(v, w) - base_edge;
                    if delta < best_delta {
                        best_delta = delta;
                        best_node = v;
                        best_pos = pos;
                    }
                }
            }

            // Only consider clusters that can be inserted without violating Tmax
            if best_delta < f64::MAX && sol.total_cost + best_delta <= pb.t_max {
                // Score = profit / marginal_cost
                // When insertion is nearly free (delta ≈ 0), the score is very
                // high — which is correct, free profit is great.
                let profit = pb.profits[cg];
                let score = if best_delta < 1e-9 {
                    profit * 1e9 // treat as "infinite" score
                } else {
                    profit / best_delta
                };
                candidates.push((score, cg, best_node, best_pos));
            }
        }

        if candidates.is_empty() {
            break; // no more feasible insertions
        }

        // Build the RCL
        let score_max = candidates
            .iter()
            .map(|c| c.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let score_min = candidates
            .iter()
            .map(|c| c.0)
            .fold(f64::INFINITY, f64::min);

        // threshold: candidates with score ≥ score_max - α*(score_max-score_min)
        let threshold = score_max - alpha * (score_max - score_min);

        let rcl: Vec<&(f64, usize, usize, usize)> =
            candidates.iter().filter(|c| c.0 >= threshold - 1e-12).collect();

        // Pick one at random from the RCL
        let chosen = rcl[rng.gen_range_usize(0, rcl.len())];
        let (_, cg, best_node, best_pos) = *chosen;

        // Apply insertion
        let u = sol.tour_nodes[best_pos - 1];
        let w = sol.tour_nodes[best_pos];
        let delta = pb.get_dist(u, best_node) + pb.get_dist(best_node, w) - pb.get_dist(u, w);

        sol.tour_nodes.insert(best_pos, best_node);
        sol.tour_clusters.insert(best_pos, cg);
        sol.total_cost += delta;
        sol.total_profit += pb.profits[cg];
        in_tour[cg] = true;
    }

    sol
}

// ─────────────────────────────────────────────────────────────────────────────
// PERTURBATION RESTART (elite-based diversification)
// ─────────────────────────────────────────────────────────────────────────────

/// Take an elite solution, randomly remove `k` clusters from it (destruction),
/// then call `grasp_construct` starting from that partial tour (repair).
///
/// This is a lightweight form of Large Neighbourhood Search (LNS) / path
/// relinking used as a restart strategy inside GRASP.
fn perturbation_restart(
    pb: &Problem,
    mut seed: Solution,
    alpha: f64,
    rng: &mut EpochRng,
    eval_count: &mut u64,
) -> Solution {
    let n_visited = seed.tour_clusters.len().saturating_sub(2);
    if n_visited == 0 {
        return grasp_construct(pb, alpha, rng, eval_count);
    }

    // Destroy 20–40% of the solution at random
    let k_min = (n_visited as f64 * 0.20).ceil() as usize;
    let k_max = (n_visited as f64 * 0.40).ceil() as usize;
    let k_max = k_max.max(k_min);
    let k = k_min + rng.gen_range_usize(0, k_max - k_min + 1);

    // Partial Fisher-Yates to select k indices to remove (1-indexed to skip depots)
    let mut indices: Vec<usize> = (1..seed.tour_clusters.len() - 1).collect();
    for i in 0..k.min(indices.len()) {
        let swap_idx = i + rng.gen_range_usize(0, indices.len() - i);
        indices.swap(i, swap_idx);
    }
    let to_remove: HashSet<usize> = indices.into_iter().take(k).collect();

    // Filter out the removed positions
    let mut new_nodes = vec![0usize];
    let mut new_clusters = vec![0usize];
    let mut new_cost = 0.0;
    let mut new_profit = 0.0;

    for pos in 1..seed.tour_clusters.len() - 1 {
        if !to_remove.contains(&pos) {
            new_nodes.push(seed.tour_nodes[pos]);
            new_clusters.push(seed.tour_clusters[pos]);
            new_profit += pb.profits[seed.tour_clusters[pos]];
        }
    }
    new_nodes.push(0);
    new_clusters.push(0);

    // Recompute cost for the partial tour
    for i in 0..new_nodes.len() - 1 {
        new_cost += pb.get_dist(new_nodes[i], new_nodes[i + 1]);
    }

    seed.tour_nodes = new_nodes;
    seed.tour_clusters = new_clusters;
    seed.total_cost = new_cost;
    seed.total_profit = new_profit;

    // Now greedily fill in remaining clusters using the same GRASP construct
    // logic, but starting from the partial solution.
    grasp_construct_from_partial(pb, seed, alpha, rng, eval_count)
}

/// Like `grasp_construct` but continues from an already-partial solution.
/// This avoids inserting clusters that are already present.
fn grasp_construct_from_partial(
    pb: &Problem,
    mut sol: Solution,
    alpha: f64,
    rng: &mut EpochRng,
    eval_count: &mut u64,
) -> Solution {
    // Mark what is already in the tour
    let mut in_tour: Vec<bool> = vec![false; pb.num_clusters];
    for &c in &sol.tour_clusters {
        in_tour[c] = true;
    }

    // Reuse the same greedy-random loop as grasp_construct
    loop {
        let mut candidates: Vec<(f64, usize, usize, usize)> = Vec::new();

        for cg in 1..pb.num_clusters {
            if in_tour[cg] {
                continue;
            }

            let mut best_delta = f64::MAX;
            let mut best_node = 0;
            let mut best_pos = 0;

            for pos in 1..sol.tour_nodes.len() {
                let u = sol.tour_nodes[pos - 1];
                let w = sol.tour_nodes[pos];
                let base_edge = pb.get_dist(u, w);

                for &v in &pb.nodes_of_cluster[cg] {
                    *eval_count += 1;
                    let delta = pb.get_dist(u, v) + pb.get_dist(v, w) - base_edge;
                    if delta < best_delta {
                        best_delta = delta;
                        best_node = v;
                        best_pos = pos;
                    }
                }
            }

            if best_delta < f64::MAX && sol.total_cost + best_delta <= pb.t_max {
                let profit = pb.profits[cg];
                let score = if best_delta < 1e-9 {
                    profit * 1e9
                } else {
                    profit / best_delta
                };
                candidates.push((score, cg, best_node, best_pos));
            }
        }

        if candidates.is_empty() {
            break;
        }

        let score_max = candidates.iter().map(|c| c.0).fold(f64::NEG_INFINITY, f64::max);
        let score_min = candidates.iter().map(|c| c.0).fold(f64::INFINITY, f64::min);
        let threshold = score_max - alpha * (score_max - score_min);
        let rcl: Vec<&(f64, usize, usize, usize)> =
            candidates.iter().filter(|c| c.0 >= threshold - 1e-12).collect();

        let chosen = rcl[rng.gen_range_usize(0, rcl.len())];
        let (_, cg, best_node, best_pos) = *chosen;

        let u = sol.tour_nodes[best_pos - 1];
        let w = sol.tour_nodes[best_pos];
        let delta = pb.get_dist(u, best_node) + pb.get_dist(best_node, w) - pb.get_dist(u, w);

        sol.tour_nodes.insert(best_pos, best_node);
        sol.tour_clusters.insert(best_pos, cg);
        sol.total_cost += delta;
        sol.total_profit += pb.profits[cg];
        in_tour[cg] = true;
    }

    sol
}

// ─────────────────────────────────────────────────────────────────────────────
// LOCAL SEARCH PHASE
// ─────────────────────────────────────────────────────────────────────────────

/// Full local search: iterate all neighbourhoods until no move improves.
///
/// Order of neighbourhoods (cheapest/fastest first to fail fast):
///   1. Or-Opt (relocate each cluster to each other position)
///   2. Swap (exchange a visited cluster for an unvisited one)
///   3. Remove-and-Reinsert (drop worst cluster, try inserting best unvisited)
///   4. DP vertex selection + 2-Opt (route refinement from phase1)
///
/// We keep looping through all four until a full pass yields no improvement.
fn full_local_search(pb: &Problem, mut sol: Solution, eval_count: &mut u64) -> Solution {
    // Apply base route improvement first so we start with a clean solution
    tour_improvement(pb, &mut sol);
    *eval_count += 1;

    loop {
        let profit_before = sol.total_profit;
        let cost_before = sol.total_cost;

        // ── Or-Opt (single-cluster relocation) ─────────────────────────────
        sol = or_opt(pb, sol, eval_count);

        // ── Swap (visited ↔ unvisited cluster) ─────────────────────────────
        sol = cluster_swap(pb, sol, eval_count);

        // ── Remove-and-Reinsert ─────────────────────────────────────────────
        sol = remove_and_reinsert(pb, sol, eval_count);

        // ── Route refinement (DP + 2-Opt) ──────────────────────────────────
        tour_improvement(pb, &mut sol);
        *eval_count += 1;

        // Converged if no neighbourhood improved
        let improved_profit = sol.total_profit > profit_before + 1e-5;
        let improved_cost =
            (sol.total_profit - profit_before).abs() < 1e-5 && sol.total_cost < cost_before - 1e-5;
        if !improved_profit && !improved_cost {
            break;
        }
    }

    sol
}

// ─────────────────────────────────────────────────────────────────────────────
// NEIGHBOURHOOD A: Or-Opt (single-cluster relocation)
// ─────────────────────────────────────────────────────────────────────────────
//
// For each visited cluster c at position `pos`, compute the cost savings from
// removing it (δ_remove < 0 means the tour shrinks).  Then try inserting it
// at every OTHER position in the tour.  Accept the best improving move.
//
// Why better than 2-opt alone:
//   2-opt only reverses segments; Or-opt can move a stop to a completely
//   different part of the route, which is often what's needed in orienteering
//   where the depot is fixed and clusters are scattered.
//
// Time complexity: O(n²) per pass where n = |tour|.
fn or_opt(pb: &Problem, mut sol: Solution, eval_count: &mut u64) -> Solution {
    let mut improved = true;
    while improved {
        improved = false;
        let n = sol.tour_nodes.len();

        // Positions 1 .. n-2 are real customer stops (0 and n-1 are depot)
        let mut best_gain = 1e-6; // must beat this threshold to accept
        let mut best_from = 0;
        let mut best_to = 0;
        let mut best_node = 0;

        for from in 1..n - 1 {
            let v = sol.tour_nodes[from];
            let u_prev = sol.tour_nodes[from - 1];
            let w_next = sol.tour_nodes[from + 1];

            // Cost reduction from removing v from its current position
            // (the edge u_prev → v → w_next is replaced by u_prev → w_next)
            let remove_saving = pb.get_dist(u_prev, v)
                + pb.get_dist(v, w_next)
                - pb.get_dist(u_prev, w_next);

            for to in 1..n - 1 {
                if to == from || to == from - 1 {
                    continue; // same position or adjacent (no change)
                }
                *eval_count += 1;

                let u_ins = sol.tour_nodes[to - 1];
                let w_ins = sol.tour_nodes[to];

                // Skip if to is on the other side of from by 1 (would be same)
                if to == from + 1 {
                    continue;
                }

                // Cost of inserting v between u_ins and w_ins
                // We need to pick the best node from the cluster at `from`
                let cg = sol.tour_clusters[from];
                let mut min_insert_delta = f64::MAX;
                let mut chosen_v = v;

                for &candidate_v in &pb.nodes_of_cluster[cg] {
                    *eval_count += 1;
                    let ins_delta = pb.get_dist(u_ins, candidate_v)
                        + pb.get_dist(candidate_v, w_ins)
                        - pb.get_dist(u_ins, w_ins);
                    if ins_delta < min_insert_delta {
                        min_insert_delta = ins_delta;
                        chosen_v = candidate_v;
                    }
                }

                // Net gain: what we save by removing − what we spend by inserting
                // Positive gain → the move improves the tour cost (more slack)
                // Since profit is unchanged (we keep the same clusters), we
                // want to maximise gain in order to free up travel budget.
                let gain = remove_saving - min_insert_delta;

                // Also accept if it strictly reduces cost while keeping profit
                if gain > best_gain {
                    best_gain = gain;
                    best_from = from;
                    best_to = to;
                    best_node = chosen_v;
                }
            }
        }

        if best_gain > 1e-6 {
            // Apply the move: remove from `best_from`, insert at `best_to`
            let cg = sol.tour_clusters.remove(best_from);
            sol.tour_nodes.remove(best_from);

            // Adjust insertion index if it was after the removal point
            let insert_at = if best_to > best_from {
                best_to - 1
            } else {
                best_to
            };

            sol.tour_nodes.insert(insert_at, best_node);
            sol.tour_clusters.insert(insert_at, cg);
            sol.recompute(pb);
            improved = true;
        }
    }

    sol
}

// ─────────────────────────────────────────────────────────────────────────────
// NEIGHBOURHOOD B: Cluster Swap (visited ↔ unvisited)
// ─────────────────────────────────────────────────────────────────────────────
//
// For each visited cluster c_out at position `pos_out`, and each unvisited
// cluster c_in, try:
//   1. Remove c_out (save its insertion cost).
//   2. Insert c_in at every position in the resulting tour.
//   3. Accept if profit(c_in) - profit(c_out) > 0 and total cost ≤ Tmax,
//      OR if same profit improvement but lower cost.
//
// This handles the case where we have the "wrong" cluster in the tour and a
// better one is left out.
//
// We use best-improvement: scan all pairs and apply the single best swap.
fn cluster_swap(pb: &Problem, mut sol: Solution, eval_count: &mut u64) -> Solution {
    let mut global_improved = true;

    while global_improved {
        global_improved = false;

        let visited_set: HashSet<usize> = sol.tour_clusters.iter().cloned().collect();

        let mut best_delta_profit = 0.0; // must improve profit (or tie with cost)
        let mut best_delta_cost = 0.0;
        let mut best_pos_out = 0;
        let mut best_cg_in = 0;
        let mut best_node_in = 0;
        let mut best_pos_in = 0;

        // Iterate over each visited customer cluster
        for pos_out in 1..sol.tour_nodes.len() - 1 {
            let c_out = sol.tour_clusters[pos_out];
            let profit_out = pb.profits[c_out];

            // Build the intermediate tour after removing c_out
            // (we don't actually mutate; compute costs symbolically)
            let u_rem = sol.tour_nodes[pos_out - 1];
            let v_rem = sol.tour_nodes[pos_out];
            let w_rem = sol.tour_nodes[pos_out + 1];
            let remove_delta = pb.get_dist(u_rem, v_rem)
                + pb.get_dist(v_rem, w_rem)
                - pb.get_dist(u_rem, w_rem);
            let cost_after_remove = sol.total_cost - remove_delta;

            // Try inserting each unvisited cluster
            for cg_in in 1..pb.num_clusters {
                if visited_set.contains(&cg_in) {
                    continue;
                }
                *eval_count += 1;

                let profit_in = pb.profits[cg_in];
                let delta_profit = profit_in - profit_out;

                // We only swap if it's at least profit-neutral;
                // pure cost reduction with same profit is handled by Or-Opt.
                if delta_profit < -1e-5 {
                    continue;
                }

                // Find cheapest insertion of cg_in into the stripped tour
                // We must iterate over all positions in the stripped tour.
                // The stripped tour has the same nodes except at pos_out where
                // u_rem now connects directly to w_rem.
                // Approximate: iterate all (pos, v) except around pos_out.
                let mut min_insert = f64::MAX;
                let mut chosen_node = 0;
                let mut chosen_pos = 0;

                // Build stripped node list on the fly (logically)
                let stripped: Vec<usize> = sol
                    .tour_nodes
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != pos_out)
                    .map(|(_, &n)| n)
                    .collect();

                for ins_pos in 1..stripped.len() {
                    let u_ins = stripped[ins_pos - 1];
                    let w_ins = stripped[ins_pos];
                    let base = pb.get_dist(u_ins, w_ins);

                    for &v_in in &pb.nodes_of_cluster[cg_in] {
                        *eval_count += 1;
                        let ins_delta = pb.get_dist(u_ins, v_in)
                            + pb.get_dist(v_in, w_ins)
                            - base;
                        if ins_delta < min_insert {
                            min_insert = ins_delta;
                            chosen_node = v_in;
                            chosen_pos = ins_pos;
                        }
                    }
                }

                if min_insert == f64::MAX {
                    continue;
                }

                let new_cost = cost_after_remove + min_insert;
                if new_cost > pb.t_max + 1e-9 {
                    continue;
                }

                let delta_cost = new_cost - sol.total_cost;

                // Accept criteria: better profit, or same profit but cheaper
                let strictly_better = delta_profit > best_delta_profit + 1e-5;
                let tie_cheaper = (delta_profit - best_delta_profit).abs() < 1e-5
                    && delta_cost < best_delta_cost - 1e-5;

                if strictly_better || tie_cheaper {
                    best_delta_profit = delta_profit;
                    best_delta_cost = delta_cost;
                    best_pos_out = pos_out;
                    best_cg_in = cg_in;
                    best_node_in = chosen_node;
                    best_pos_in = chosen_pos;
                }
            }
        }

        // Apply the best swap found
        if best_delta_profit > -1e-5
            && (best_delta_profit > 1e-5 || best_delta_cost < -1e-5)
            && best_pos_out > 0
        {
            // Remove the old cluster
            sol.tour_nodes.remove(best_pos_out);
            sol.tour_clusters.remove(best_pos_out);

            // Adjust insertion position if needed
            let ins_at = if best_pos_in > best_pos_out {
                best_pos_in - 1
            } else {
                best_pos_in
            };

            sol.tour_nodes.insert(ins_at, best_node_in);
            sol.tour_clusters.insert(ins_at, best_cg_in);
            sol.recompute(pb);

            // Run DP + 2-opt to clean up the new route
            optimize_vertices_dp(pb, &mut sol);
            optimize_routing_2opt(pb, &mut sol);
            *eval_count += 1;

            global_improved = true;
        }
    }

    sol
}

// ─────────────────────────────────────────────────────────────────────────────
// NEIGHBOURHOOD C: Remove-and-Reinsert (worst-cluster replacement)
// ─────────────────────────────────────────────────────────────────────────────
//
// Identify the cluster with the worst "efficiency" score in the current tour:
//   efficiency = profit / (cost_contribution)
// where cost_contribution is how much cost that cluster adds vs. the direct
// edge between its predecessor and successor.
//
// Remove it, freeing travel budget.  Then greedily try to insert the best
// unvisited cluster (by profit/insertion_cost ratio) using that freed budget.
//
// This is a targeted improvement when one cluster is "clogging" the tour by
// consuming too much budget relative to its profit.
fn remove_and_reinsert(pb: &Problem, mut sol: Solution, eval_count: &mut u64) -> Solution {
    if sol.tour_clusters.len() <= 2 {
        return sol; // no customer clusters to remove
    }

    let n = sol.tour_clusters.len();

    // Find the cluster with the worst efficiency score
    let mut worst_eff = f64::MAX;
    let mut worst_pos = 0;

    for pos in 1..n - 1 {
        let v = sol.tour_nodes[pos];
        let u = sol.tour_nodes[pos - 1];
        let w = sol.tour_nodes[pos + 1];
        let cg = sol.tour_clusters[pos];

        // How much does this cluster cost us in travel?
        let cost_contribution = pb.get_dist(u, v) + pb.get_dist(v, w) - pb.get_dist(u, w);
        let profit = pb.profits[cg];

        // Efficiency: profit per unit of extra travel introduced.
        // Clusters with low efficiency are candidates for removal.
        let eff = if cost_contribution < 1e-9 {
            profit * 1e9 // free cluster → keep it
        } else {
            profit / cost_contribution
        };

        if eff < worst_eff {
            worst_eff = eff;
            worst_pos = pos;
        }
    }

    if worst_pos == 0 {
        return sol;
    }

    // Tentatively remove the worst cluster
    let removed_v = sol.tour_nodes[worst_pos];
    let removed_c = sol.tour_clusters[worst_pos];
    let u_rem = sol.tour_nodes[worst_pos - 1];
    let w_rem = sol.tour_nodes[worst_pos + 1];
    let remove_saving = pb.get_dist(u_rem, removed_v)
        + pb.get_dist(removed_v, w_rem)
        - pb.get_dist(u_rem, w_rem);

    let cost_without = sol.total_cost - remove_saving;
    let profit_without = sol.total_profit - pb.profits[removed_c];

    // Build the visited set without removed_c
    let mut visited_without: HashSet<usize> = sol.tour_clusters.iter().cloned().collect();
    visited_without.remove(&removed_c);

    // Find the best unvisited cluster to insert instead
    let mut best_score = f64::NEG_INFINITY;
    let mut best_cg = 0;
    let mut best_node = 0;
    let mut best_pos_in = 0;
    let mut best_insert_cost = 0.0;

    // Build stripped tour
    let stripped_nodes: Vec<usize> = sol
        .tour_nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != worst_pos)
        .map(|(_, &n)| n)
        .collect();

    for cg in 1..pb.num_clusters {
        if visited_without.contains(&cg) {
            continue;
        }
        *eval_count += 1;

        let profit_in = pb.profits[cg];

        // Find cheapest insertion position into the stripped tour
        let mut min_delta = f64::MAX;
        let mut chosen_node = 0;
        let mut chosen_pos = 0;

        for pos in 1..stripped_nodes.len() {
            let u = stripped_nodes[pos - 1];
            let w = stripped_nodes[pos];
            let base = pb.get_dist(u, w);

            for &v in &pb.nodes_of_cluster[cg] {
                *eval_count += 1;
                let delta = pb.get_dist(u, v) + pb.get_dist(v, w) - base;
                if delta < min_delta {
                    min_delta = delta;
                    chosen_node = v;
                    chosen_pos = pos;
                }
            }
        }

        if min_delta == f64::MAX || cost_without + min_delta > pb.t_max + 1e-9 {
            continue;
        }

        let score = if min_delta < 1e-9 {
            profit_in * 1e9
        } else {
            profit_in / min_delta
        };

        if score > best_score {
            best_score = score;
            best_cg = cg;
            best_node = chosen_node;
            best_pos_in = chosen_pos;
            best_insert_cost = min_delta;
        }
    }

    // Decide: is the replacement better than keeping the original?
    // "Better" = higher profit, or same profit but lower cost.
    if best_cg > 0 {
        let new_profit = profit_without + pb.profits[best_cg];
        let new_cost = cost_without + best_insert_cost;

        let strictly_better_profit = new_profit > sol.total_profit + 1e-5;
        let tie_cheaper =
            (new_profit - sol.total_profit).abs() < 1e-5 && new_cost < sol.total_cost - 1e-5;

        if strictly_better_profit || tie_cheaper {
            // Apply the replacement
            sol.tour_nodes.remove(worst_pos);
            sol.tour_clusters.remove(worst_pos);

            let ins_at = if best_pos_in > worst_pos {
                best_pos_in - 1
            } else {
                best_pos_in
            };

            sol.tour_nodes.insert(ins_at, best_node);
            sol.tour_clusters.insert(ins_at, best_cg);
            sol.recompute(pb);
        }
        // If no better replacement, also try inserting cg_best WITHOUT removing
        // anything (just fill slack).  This is handled by grasp_construct already,
        // so we skip here to avoid redundancy.
    } else {
        // No replacement found — check if simply removing the worst cluster
        // frees enough budget to insert a high-value cluster we couldn't
        // afford before. Try inserting ANY previously infeasible cluster.
        let mut inserted = false;
        for cg in 1..pb.num_clusters {
            if visited_without.contains(&cg) {
                continue;
            }
            *eval_count += 1;

            let mut min_delta = f64::MAX;
            let mut chosen_node = 0;
            let mut chosen_pos_ins = 0;

            for pos in 1..stripped_nodes.len() {
                let u = stripped_nodes[pos - 1];
                let w = stripped_nodes[pos];
                let base = pb.get_dist(u, w);
                for &v in &pb.nodes_of_cluster[cg] {
                    *eval_count += 1;
                    let delta = pb.get_dist(u, v) + pb.get_dist(v, w) - base;
                    if delta < min_delta {
                        min_delta = delta;
                        chosen_node = v;
                        chosen_pos_ins = pos;
                    }
                }
            }

            if min_delta < f64::MAX && cost_without + min_delta <= pb.t_max + 1e-9 {
                let new_profit = profit_without + pb.profits[cg];
                if new_profit > sol.total_profit + 1e-5 {
                    sol.tour_nodes.remove(worst_pos);
                    sol.tour_clusters.remove(worst_pos);

                    let ins_at = if chosen_pos_ins > worst_pos {
                        chosen_pos_ins - 1
                    } else {
                        chosen_pos_ins
                    };
                    sol.tour_nodes.insert(ins_at, chosen_node);
                    sol.tour_clusters.insert(ins_at, cg);
                    sol.recompute(pb);
                    inserted = true;
                    break;
                }
            }
        }
        // If nothing works, keep the solution unchanged
        let _ = inserted;
    }

    sol
}

// ─────────────────────────────────────────────────────────────────────────────
// ELITE POOL MANAGEMENT
// ─────────────────────────────────────────────────────────────────────────────
//
// The elite pool stores the best-K distinct solutions found so far.
// "Distinct" = different profit or substantially different cost.
// When full, evict the worst solution (lowest profit, or highest cost on tie).
fn update_elite(elite: &mut Vec<Solution>, sol: Solution, max_size: usize) {
    // Check if it's distinct enough from existing members
    for existing in elite.iter() {
        let same_profit = (existing.total_profit - sol.total_profit).abs() < 1e-5;
        let same_cost = (existing.total_cost - sol.total_cost).abs() < 1.0;
        if same_profit && same_cost {
            return; // too similar, skip
        }
    }

    elite.push(sol);

    if elite.len() > max_size {
        // Evict the worst solution (lowest profit; break ties by highest cost)
        let worst_idx = elite
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.total_profit
                    .partial_cmp(&b.total_profit)
                    .unwrap()
                    .then(b.total_cost.partial_cmp(&a.total_cost).unwrap())
            })
            .map(|(i, _)| i)
            .unwrap_or(0);
        elite.swap_remove(worst_idx);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CONVENIENCE WRAPPER matching your existing Tabu Search interface
// ─────────────────────────────────────────────────────────────────────────────

/// Convenience wrapper with sensible defaults. Call this from `main.rs`.
pub fn grasp_default(
    pb: &Problem,
    max_time_secs: u64,
    max_evals: u64,
    start_time: Instant,
) -> (Solution, u64) {
    grasp(
        pb,
        0.15,  // α_start: slightly randomised greedy
        7,     // elite pool size
        max_time_secs,
        max_evals,
        start_time,
    )
}