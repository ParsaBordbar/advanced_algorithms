use std::env;
use std::time::Instant;

use sop_simulated_annealing::feasibility::is_feasible;
use sop_simulated_annealing::io::output::write_solution_file;
use sop_simulated_annealing::io::parser::parse_instance;
use sop_simulated_annealing::rng::EpochRng;
use sop_simulated_annealing::solver::phase1::phase1_construction;
// use sop_simulated_annealing::solver::init::random_feasible_initial_solution; // uncomment to use random init sol
use sop_simulated_annealing::configs::Config;
use sop_simulated_annealing::solver::simulated_annealing::{
    RunStats, StopCriteria, simulated_annealing,
};

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
    let max_time_secs: u64 = args[2].parse().expect("Invalid max-time-seconds");
    let max_evals: u64 = args[3].parse().expect("Invalid max-evaluations");

    let Config {
        t_start,
        t_final,
        alpha,
        epoch_length,
    } = Config::default();

    let start = Instant::now();

    let mut rng = EpochRng::new();

    let pb = parse_instance(instance_path);
    let initial_sol = phase1_construction(&pb);
    // let initial_sol = random_feasible_initial_solution(&pb,  &mut rng); //uncomment to use random init sol

    let stop = StopCriteria {
        max_time_secs,
        max_evals,
    };
    let mut stats = RunStats { eval_count: 0 };

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
    }

    match write_solution_file(instance_path, &best_sol) {
        Ok(path) => println!("Solution written to ./src/data/output{}", path),
        Err(e) => eprintln!("Failed to write output file: {}", e),
    }
}
