use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::models::Solution;

pub fn write_solution_file(instance_path: &str, sol: &Solution) -> std::io::Result<String> {
    let stem = Path::new(instance_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let out_dir = Path::new("src/data/output");
    fs::create_dir_all(out_dir)?; // ensures the dir exists

    let out_path: PathBuf = out_dir.join(format!("{}_output.txt", stem));
    let mut f = File::create(&out_path)?;

    let customers: Vec<String> = sol.tour_nodes[1..sol.tour_nodes.len() - 1]
        .iter()
        .map(|&n| (n + 1).to_string())
        .collect();

    writeln!(f, "{}", customers.join(" "))?;
    writeln!(f, "Travel_Time: {}", sol.total_cost.round() as i64)?;
    writeln!(f, "Profit: {}", sol.total_profit.round() as i64)?;

    Ok(out_path.display().to_string())
}
