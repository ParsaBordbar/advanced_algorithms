#[derive(Clone)]
pub struct Problem {
    pub num_nodes: usize,
    pub num_clusters: usize,
    pub t_max: f64,
    pub dist: Vec<f64>, // Flattened 1D matrix for cache speed
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
