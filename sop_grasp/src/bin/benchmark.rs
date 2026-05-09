use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

use sop_grasp::io::parser::parse_instance;
use sop_grasp::rng::EpochRng;
// Import the new GRASP function (adjust the path if necessary based on your file structure)
use sop_grasp::solver::phase2_grasp::grasp;
use sop_grasp::configs::{DataPath};

struct BenchmarkResult {
    instance: String,
    best_profit: f64,
    best_cost: f64,
    avg_time_ms: u128,
}

fn main() {
    let dp = DataPath::default();
    let instances_dir = format!("{}/instances", dp.data_dir);
    let output_file = format!("{}{}/benchmark_results.csv", dp.data_dir, dp.output_dir);

    let paths = fs::read_dir(instances_dir).expect("Failed to read directory");
    let mut files: Vec<_> = paths.filter_map(Result::ok).collect();

    files.sort_by_key(|dir| dir.path());

    // GRASP Configuration
    let max_iterations: usize = 200; // Define how many iterations GRASP should run
    let alpha: f64 = 0.3;            // RCL randomness factor (0.0 = greedy, 1.0 = random)
    let num_runs = 3; 

    let mut all_results: Vec<BenchmarkResult> = Vec::new();

    for entry in files {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("sop") {
            let filename = path.file_name().unwrap().to_str().unwrap().to_string();
            println!("Processing {} ({} runs)...", filename, num_runs);

            let problem = parse_instance(path.to_str().unwrap());

            let mut best_profit = -1.0;
            let mut best_cost = f64::MAX;
            let mut total_time = 0;

            for _ in 0..num_runs {
                let start = Instant::now();
                let mut rng = EpochRng::new();

                // Run GRASP instead of SA
                let sol = grasp(&problem, &mut rng, max_iterations, alpha);

                total_time += start.elapsed().as_millis();

                if sol.total_profit > best_profit
                    || (sol.total_profit == best_profit && sol.total_cost < best_cost)
                {
                    best_profit = sol.total_profit;
                    best_cost = sol.total_cost;
                }
            }

            all_results.push(BenchmarkResult {
                instance: filename,
                best_profit,
                best_cost,
                avg_time_ms: total_time / (num_runs as u128),
            });
        }
    }

    all_results.sort_by_key(|res| {
        res.instance
            .strip_prefix("instance_")
            .and_then(|s| s.strip_suffix(".sop"))
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(usize::MAX)
    });

    let mut csv = File::create(&output_file).expect("Failed to create CSV");
    writeln!(csv, "Instance,Best_Profit,Best_Cost,Avg_Time_ms").unwrap();

    for res in &all_results {
        writeln!(
            csv,
            "{},{},{},{}",
            res.instance, res.best_profit, res.best_cost, res.avg_time_ms
        )
        .unwrap();
    }

    println!(
        "Benchmarking complete! {} instances evaluated and saved to {}",
        all_results.len(),
        output_file
    );
}
