use crate::models::{Problem, Solution};


pub fn phase1_construction(pb: &Problem) -> Solution {
    let mut sol = Solution::new();
    let mut unvisited_clusters: Vec<usize> = (1..pb.num_clusters).collect();
    
    // Sort clusters by descending profit
    unvisited_clusters.sort_unstable_by(|&a, &b| pb.profits[b].partial_cmp(&pb.profits[a]).unwrap());

    for &cg in &unvisited_clusters {
        let u = sol.tour_nodes[sol.tour_nodes.len() - 2];
        let mut best_v = None;
        let mut min_cost_add = f64::MAX;

        for &v in &pb.nodes_of_cluster[cg] {
            let cost_add = pb.get_dist(u, v) + pb.get_dist(v, 0) - pb.get_dist(u, 0);
            if sol.total_cost + cost_add <= pb.t_max {
                if cost_add < min_cost_add {
                    min_cost_add = cost_add;
                    best_v = Some(v);
                }
            }
        }

        if let Some(v) = best_v {
            sol.tour_nodes.insert(sol.tour_nodes.len() - 1, v);
            sol.tour_clusters.insert(sol.tour_clusters.len() - 1, cg);
            sol.total_cost += min_cost_add;
            sol.total_profit += pb.profits[cg];
        }
    }

    tour_improvement(pb, &mut sol);
    sol
}

// 4. TOUR IMPROVEMENT (Step 1 & Step 2)
pub fn tour_improvement(pb: &Problem, sol: &mut Solution) {
    let mut improved = true;
    while improved {
        improved = false;
        let cost_before = sol.total_cost;
        
        optimize_vertices_dp(pb, sol);
        optimize_routing_2opt(pb, sol);

        if sol.total_cost < cost_before - 1e-5 {
            improved = true;
        }
    }
}

pub fn optimize_vertices_dp(pb: &Problem, sol: &mut Solution) {
    let n_clusters = sol.tour_clusters.len();
    if n_clusters <= 2 { return; }

    let mut dp = vec![vec![f64::MAX; pb.num_nodes]; n_clusters];
    let mut parent = vec![vec![0usize; pb.num_nodes]; n_clusters];

    dp[0][0] = 0.0;

    for i in 1..n_clusters {
        let prev_cluster = sol.tour_clusters[i - 1];
        let curr_cluster = sol.tour_clusters[i];

        for &u in &pb.nodes_of_cluster[prev_cluster] {
            if dp[i - 1][u] == f64::MAX { continue; }

            for &v in &pb.nodes_of_cluster[curr_cluster] {
                let cost = dp[i - 1][u] + pb.get_dist(u, v);
                if cost < dp[i][v] {
                    dp[i][v] = cost;
                    parent[i][v] = u;
                }
            }
        }
    }

    let mut curr_node = 0;
    for i in (1..n_clusters).rev() {
        sol.tour_nodes[i] = curr_node;
        curr_node = parent[i][curr_node];
    }
    sol.tour_nodes[0] = 0;
    sol.recompute(pb);
}

pub fn optimize_routing_2opt(pb: &Problem, sol: &mut Solution) {
    let mut improved = true;
    let n = sol.tour_nodes.len();
    while improved {
        improved = false;
        for i in 1..n - 2 {
            for j in i + 1..n - 1 {
                let n1 = sol.tour_nodes[i - 1];
                let n2 = sol.tour_nodes[i];
                let n3 = sol.tour_nodes[j];
                let n4 = sol.tour_nodes[j + 1];

                let current_dist = pb.get_dist(n1, n2) + pb.get_dist(n3, n4);
                let new_dist = pb.get_dist(n1, n3) + pb.get_dist(n2, n4);

                if new_dist < current_dist - 1e-5 {
                    sol.tour_nodes[i..=j].reverse();
                    sol.tour_clusters[i..=j].reverse();
                    sol.recompute(pb);
                    improved = true;
                }
            }
        }
    }
}
