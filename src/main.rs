use std::env;
use std::time::Instant;

use sop_simulated_annealing::parser::parse_instance;
use sop_simulated_annealing::solver::phase1::phase1_construction;
use sop_simulated_annealing::solver::simulated_annealing::simulated_annealing;
use sop_simulated_annealing::rng::EpochRng; 
use sop_simulated_annealing::models::Solution; // Added to create an empty solution

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <instance_path> <epoch_length> <alpha> [--skip-phase1]", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    let epoch_length: usize = args[2].parse().expect("Invalid epoch_length");
    let alpha: f64 = args[3].parse().expect("Invalid alpha (should be a float like 0.95)");
    
    // Check if the skip flag is present in the arguments
    let skip_phase1 = args.contains(&"--skip-phase1".to_string());

    let t_start = 1000.0;
    let t_final = 0.1;

    let start_time = Instant::now();

    println!("Parsing instance: {}", file_path);
    let pb = parse_instance(file_path);

    let initial_sol = if skip_phase1 {
        println!("Skipping Phase 1. Starting from empty solution...");
        let mut sol = Solution::new();
        sol.update_nodes_greedy(&pb);
        sol.recompute(&pb);
        sol
    } else {
        println!("Starting Phase 1 (Constructive Heuristic)...");
        let sol = phase1_construction(&pb);
        println!("Phase 1 Complete -> Profit: {}, Cost: {}", sol.total_profit, sol.total_cost);
        sol
    };

    println!("Starting Phase 2 (Simulated Annealing)...");
    
    let mut rng = EpochRng::new();

    let best_sol = simulated_annealing(
        &pb, 
        initial_sol, 
        t_start, 
        t_final, 
        alpha, 
        epoch_length, 
        &mut rng
    );

    let duration = start_time.elapsed();

    println!("\n=== FINAL RESULTS ===");
    println!("Execution Time: {:.2?}", duration);
    println!("Best Profit: {}", best_sol.total_profit);
    println!("Best Cost: {}", best_sol.total_cost);
    println!("Clusters Visited: {:?}", best_sol.tour_clusters);
}
