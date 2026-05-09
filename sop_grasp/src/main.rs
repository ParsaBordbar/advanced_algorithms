use sop_grasp::io::input::parse_instance;
use sop_grasp::io::output::write_solution_file;
use sop_grasp::feasibility::is_feasible;
use sop_grasp::rng::EpochRng;
use sop_grasp::solver::phase2_grasp::grasp;

use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <instance_file> <max_time_secs> <max_evals>", args[0]);
        std::process::exit(1);
    }

    let instance_path = &args[1];
    let max_time_secs: u64 = args[2].parse().unwrap_or(0);
    let max_evals: u64 = args[3].parse().unwrap_or(0);
    
    // Read the problem
    let pb = parse_instance(instance_path);
    
    // Initialize RNG
    let mut rng = EpochRng::new();

    println!("Solving {}", instance_path);
    println!("Time Limit: {}s, Eval Limit: {}", max_time_secs, max_evals);

    let start_time = Instant::now();
    let alpha = 0.3; // GRASP greediness factor

    // FIX: Pass the correct parameters and destructure the return tuple into (Solution, u64)
    let (best_sol, total_evals) = grasp(
        &pb, 
        &mut rng, 
        alpha, 
        max_time_secs, 
        max_evals, 
        start_time
    );

    let duration = start_time.elapsed();

    // FIX: best_sol is now directly a Solution object, so references work correctly
    let feasible = is_feasible(&pb, &best_sol);

    println!("--- GRASP Results ---");
    // FIX: fields are accessed directly on best_sol, not a tuple
    println!("Best Profit: {}", best_sol.total_profit);
    println!("Best Cost: {}", best_sol.total_cost);
    println!("Total Evaluations: {}", total_evals);
    println!("Feasible: {}", feasible);
    println!("Time Taken: {:.2?}", duration);

    // FIX: Pass both the instance_path and the solution to the writer
    match write_solution_file(instance_path, &best_sol) {
        Ok(path) => println!("Saved solution to {}", path),
        Err(e) => eprintln!("Error saving solution: {}", e),
    }
}
