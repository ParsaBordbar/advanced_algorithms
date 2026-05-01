#[derive(Debug, Clone)]
pub struct Config {
    pub lambda: f64,
    pub beta: f64,
    pub alpha: usize,
    pub max_iter: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lambda: 0.5,
            beta: 0.05,
            alpha: 10,
            max_iter: 1000,
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
