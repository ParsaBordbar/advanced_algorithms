use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

use sop_simulated_annealing::parser::parse_instance;
use sop_simulated_annealing::solver::phase1::phase1_construction;
use sop_simulated_annealing::solver::simulated_annealing::simulated_annealing;
use sop_simulated_annealing::rng::EpochRng;

// A struct to hold our results in memory so we can sort them later
struct BenchmarkResult {
    instance: String,
    best_profit: f64,
    best_cost: f64,
    avg_time_ms: u128,
}

fn main() {
    let instances_dir = "src/instances";
    let output_file = "benchmark_results.csv";
    
    // Read and collect paths
    let paths = fs::read_dir(instances_dir).expect("Failed to read directory");
    let mut files: Vec<_> = paths.filter_map(Result::ok).collect();
    
    // Sort files alphabetically so they are processed in a consistent order
    files.sort_by_key(|dir| dir.path());

    let t_start = 1000.0;
    let t_final = 0.1;
    let alpha = 0.999;
    let epoch_length = 1000;
    let num_runs = 3; // Number of times to run each instance

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

            // Run the solver 3 times
            for _ in 0..num_runs {
                let start = Instant::now();
                
                let initial_sol = phase1_construction(&problem);
                let mut rng = EpochRng::new();
                
                let sol = simulated_annealing(
                    &problem, 
                    initial_sol, 
                    t_start, 
                    t_final, 
                    alpha, 
                    epoch_length,
                    &mut rng 
                );
                
                total_time += start.elapsed().as_millis();

                // Keep the best solution (Highest profit, tie-breaker: lowest cost)
                if sol.total_profit > best_profit || (sol.total_profit == best_profit && sol.total_cost < best_cost) {
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

    // SORTING THE RESULTS: 
    // Here we sort descending by Profit. 
    // If you want to sort alphabetically by instance name, change to:
    // all_results.sort_by(|a, b| a.instance.cmp(&b.instance));

        all_results.sort_by_key(|res| {
        // Extract "27" from "instance_27.sop"
        res.instance
            .strip_prefix("instance_")
            .and_then(|s| s.strip_suffix(".sop"))
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(usize::MAX) // Fallback if parsing fails
    });
    // Write sorted results to CSV
    let mut csv = File::create(output_file).expect("Failed to create CSV");
    writeln!(csv, "Instance,Best_Profit,Best_Cost,Avg_Time_ms").unwrap();
    
    for res in &all_results {
        writeln!(
            csv, 
            "{},{},{},{}", 
            res.instance, 
            res.best_profit, 
            res.best_cost, 
            res.avg_time_ms
        ).unwrap();
    }
    
    println!("Benchmarking complete! {} instances evaluated and saved to {}", all_results.len(), output_file);
}
