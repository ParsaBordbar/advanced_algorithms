use std::env;
use std::time::Instant;

use sop_tabu::configs::Config;
use sop_tabu::io::parser::parse_instance;
use sop_tabu::io::output::write_solution_file;

// use sop_tabu::solver::phase1::phase1_construction;
use sop_tabu::solver::phase1_rand::random_feasible_initial_solution;
use sop_tabu::rng::EpochRng;

use sop_tabu::solver::phase2::tabu_search;
use sop_tabu::feasibility::is_feasible;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "\nUsage: {} <instance-file-path> <max-time-seconds> <max-evaluations>\n",
            args[0]
        );
        std::process::exit(1);
    }

    let instance_path = &args[1];
    let max_time_secs: u64 = args[2]
        .parse()
        .expect("Invalid value for <max-time-seconds>");
    let max_evals: u64 = args[3]
        .parse()
        .expect("Invalid value for <max-evaluations>");

    let config = Config::default();
    let start_time = Instant::now();

    println!("Parsing instance: {}", instance_path);
    let pb = parse_instance(instance_path);

    println!("Starting Phase 1 (Initialization)...");
    
    // let initial_sol = phase1_construction(&pb);

    let mut epoch_rng = EpochRng::new();
    let initial_sol = random_feasible_initial_solution(&pb, &mut epoch_rng);

    println!("Phase 1 Complete -> Profit: {}, Cost: {}", initial_sol.total_profit, initial_sol.total_cost);

    println!("Starting Phase 2 (Tabu Search)...");
    let (best_sol, final_eval_count) = tabu_search(
        &pb,
        initial_sol,
        config.lambda,
        config.beta,
        config.alpha,
        max_time_secs,
        max_evals,
        start_time,
    );

    let duration = start_time.elapsed();

    // Check feasibility of the final solution
    let feasible = is_feasible(&pb, &best_sol); 

    println!("\n=== FINAL RESULTS ===");
    println!("Execution Time: {:.2?}", duration);
    println!("Objective Evaluations: {}", final_eval_count);
    println!("Best Profit: {}", best_sol.total_profit);
    println!("Best Cost: {}", best_sol.total_cost);
    println!("Feasible: {}", feasible);

    match write_solution_file(instance_path, &best_sol) {
        Ok(path) => println!("Solution successfully written to: {}", path),
        Err(e) => eprintln!("Failed to write solution file: {}", e),
    }
}
