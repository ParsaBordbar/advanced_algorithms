use std::env;
use std::time::Instant;

use sop_simulated_annealing::feasibility::is_feasible;
use sop_simulated_annealing::io::output::write_solution_file;
use sop_simulated_annealing::io::parser::parse_instance;
use sop_simulated_annealing::rng::EpochRng;
use sop_simulated_annealing::solver::phase1::phase1_construction;
use sop_simulated_annealing::solver::simulated_annealing::{simulated_annealing, RunStats, StopCriteria};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!(
            "Usage: {} <instance-file-path> <max-time-seconds> <max-evaluations>",
            args[0]
        );
        std::process::exit(1);
    }

    let instance_path = &args[1];
    let max_time_secs: u64 = args[2].parse().expect("Invalid max-time-seconds");
    let max_evals: u64 = args[3].parse().expect("Invalid max-evaluations");

    // Single fixed parameter set for all instances
    let t_start = 1000.0;
    let t_final = 0.001;
    let alpha = 0.9997;
    let epoch_length = 5000;

    let start = Instant::now();

    let pb = parse_instance(instance_path);
    let initial_sol = phase1_construction(&pb);

    let stop = StopCriteria { max_time_secs, max_evals };
    let mut stats = RunStats { eval_count: 0 };
    let mut rng = EpochRng::new();

    let best_sol = simulated_annealing(
        &pb,
        initial_sol,
        t_start,
        t_final,
        alpha,
        epoch_length,
        &mut rng,
        &stop,
        &mut stats,
    );

    let feasible = is_feasible(&pb, &best_sol);

    println!("Execution Time: {:.2?}", start.elapsed());
    println!("Objective Evaluations: {}", stats.eval_count);
    println!("Best Profit: {}", best_sol.total_profit);
    println!("Best Cost: {}", best_sol.total_cost);
    println!("Feasible: {}", feasible);

    if !feasible {
        eprintln!("Warning: produced solution is infeasible by checker.");
        // Depending on your preference:
        // std::process::exit(2);
    }

    match write_solution_file(instance_path, &best_sol) {
        Ok(_path) => println!("Solution written to {}", "./src/data/output{path}"),
        Err(e) => eprintln!("Failed to write output file: {}", e),
    }
}
