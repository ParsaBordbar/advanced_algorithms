#[derive(Clone)]
pub struct Problem {
    pub num_nodes: usize,
    pub num_clusters: usize,
    pub t_max: f64,
    pub dist: Vec<f64>,
    pub cluster_of_node: Vec<usize>,
    pub nodes_of_cluster: Vec<Vec<usize>>,
    pub profits: Vec<f64>,
}

impl Problem {
    #[inline(always)]
    pub fn get_dist(&self, i: usize, j: usize) -> f64 {
        self.dist[i * self.num_nodes + j]
    }
}

#[derive(Clone, Debug)]
pub struct Solution {
    pub tour_nodes: Vec<usize>,
    pub tour_clusters: Vec<usize>,
    pub total_profit: f64,
    pub total_cost: f64,
}

impl Solution {
    pub fn new() -> Self {
        Self {
            tour_nodes: vec![0, 0],
            tour_clusters: vec![0, 0],
            total_profit: 0.0,
            total_cost: 0.0,
        }
    }

    pub fn update_nodes_greedy(&mut self, pb: &Problem) {
        let mut new_nodes = Vec::with_capacity(self.tour_clusters.len());
        new_nodes.push(0); // Start at depot (node 0)

        for i in 1..(self.tour_clusters.len() - 1) {
            let prev_node = new_nodes[i - 1];
            let current_cluster = self.tour_clusters[i];

            let mut best_node = 0;
            let mut min_dist = f64::MAX;

            // Find the closest node in the current cluster to the previous node
            for &node in &pb.nodes_of_cluster[current_cluster] {
                let dist = pb.get_dist(prev_node, node);
                if dist < min_dist {
                    min_dist = dist;
                    best_node = node;
                }
            }
            new_nodes.push(best_node);
        }

        new_nodes.push(0); // End at depot (node 0)
        self.tour_nodes = new_nodes;
    }

    pub fn recompute(&mut self, pb: &Problem) {
        self.total_cost = 0.0;
        self.total_profit = 0.0;
        for i in 0..(self.tour_nodes.len() - 1) {
            self.total_cost += pb.get_dist(self.tour_nodes[i], self.tour_nodes[i + 1]);
        }
        for &c in self.tour_clusters.iter().take(self.tour_clusters.len() - 1) {
            if c != 0 {
                self.total_profit += pb.profits[c];
            }
        }
    }
}
