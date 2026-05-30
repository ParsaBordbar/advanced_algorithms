/// Genetic Algorithm configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Population size (λ in ES terminology)
    pub population_size: usize,
    /// Number of parents selected each generation (μ)
    pub num_parents: usize,
    /// Probability of crossover occurring (0.0 - 1.0)
    pub crossover_rate: f64,
    /// Probability of mutation occurring (0.0 - 1.0)
    pub mutation_rate: f64,
    /// Tournament size for parent selection
    pub tournament_size: usize,
    /// Elitism: number of best individuals carried over unchanged
    pub elite_count: usize,

    /// ---- Memetic mode settings ----
    /// Whether to run local search after each offspring generation
    pub memetic: bool,
    /// How often (every N generations) to apply full local search to entire population
    pub local_search_freq: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            population_size: 80,
            num_parents: 40,
            crossover_rate: 0.85,
            mutation_rate: 0.15,
            tournament_size: 5,
            elite_count: 2,
            memetic: false,
            local_search_freq: 10,
        }
    }
}

impl Config {
    /// Memetic variant: tighter local search integration
    pub fn memetic_default() -> Self {
        Self {
            population_size: 50,
            num_parents: 25,
            crossover_rate: 0.85,
            mutation_rate: 0.20,
            tournament_size: 4,
            elite_count: 2,
            memetic: true,
            local_search_freq: 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataPath {
    pub data_dir: String,
    pub output_dir: String,
    pub instances_dir: String,
}

impl Default for DataPath {
    fn default() -> Self {
        Self {
            data_dir: "src/data".to_string(),
            output_dir: "/output".to_string(),
            instances_dir: "src/data/instances".to_string(),
        }
    }
}
