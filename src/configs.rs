#[derive(Debug, Clone)]
pub struct Config {
    pub t_start: f64,
    pub t_final: f64,
    pub alpha: f64,
    pub epoch_length: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            t_start: 16.0,
            t_final: 0.001,
            alpha: 0.93,
            epoch_length: 80_000,
        }
    }
}
