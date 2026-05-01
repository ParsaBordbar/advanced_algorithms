use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

use sop_tabu::configs::{Config, DataPath};
use sop_tabu::io::parser::parse_instance;
use sop_tabu::solver::phase1::phase1_construction;
use sop_tabu::solver::phase2::tabu_search;
use sop_tabu::solver::phase1_rand::random_feasible_initial_solution;
use sop_tabu::rng::EpochRng;

fn main() {
    let data_paths = DataPath::default();
    let config = Config::default();
    
    let output_file = format!("{}{}/benchmark_results.csv", data_paths.data_dir, data_paths.output_dir);
    
    let mut csv = File::create(&output_file).expect("Failed to create CSV");
    writeln!(csv, "Instance,bestProfit,averageProfit,avgCost,avgTime").unwrap();

    let mut paths: Vec<_> = fs::read_dir(&data_paths.instances_dir)
        .expect("Failed to read directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("sop"))
        .collect();

    paths.sort_by_key(|path| {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let digits: String = stem.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.parse::<u32>().unwrap_or(0)
    });

    let runs = 3;
    let max_time_secs = 0; // 5 minutes per run
    let max_evals = 0;

    for path in paths {
        let filename = path.file_name().unwrap().to_str().unwrap();
        println!("Processing {}...", filename);

        let problem = parse_instance(path.to_str().unwrap());
        
        let mut best_profit = 0.0;
        let mut total_profit = 0.0;
        let mut total_cost = 0.0;
        let mut total_time_ms = 0;

        for _ in 0..runs {
            let start = Instant::now();
            
            // let initial_sol = phase1_construction(&problem);
            let mut epoch_rng = EpochRng::new();
            let initial_sol = random_feasible_initial_solution(&problem, &mut epoch_rng);
            
            // Updated to unpack the tuple and pass the new arguments
            let (best_sol, _evals) = tabu_search(
                &problem, 
                initial_sol, 
                config.lambda, 
                config.beta, 
                config.alpha, 
                max_time_secs,
                max_evals,
                start
            );
            
            let duration = start.elapsed().as_millis();

            let current_profit = best_sol.total_profit as f64;
            let current_cost = best_sol.total_cost as f64;

            if current_profit > best_profit {
                best_profit = current_profit;
            }
            
            total_profit += current_profit;
            total_cost += current_cost;
            total_time_ms += duration;
        }

        let avg_profit = total_profit / (runs as f64);
        let avg_cost = total_cost / (runs as f64);
        let avg_time = (total_time_ms as f64) / (runs as f64);

        writeln!(
            csv, 
            "{},{},{:.2},{:.2},{:.2}", 
            filename, 
            best_profit, 
            avg_profit, 
            avg_cost, 
            avg_time
        ).unwrap();
    }
    
    println!("Benchmarking complete. Results saved to {}", &output_file);
}
