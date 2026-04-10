use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::models::Problem;

pub fn parse_instance(file_path: &str) -> Problem {
    let file = File::open(file_path).expect("Could not open file");
    let reader = BufReader::new(file);

    let mut num_nodes = 0;
    let mut num_clusters = 0;
    let mut t_max = 0.0;
    let mut coords: Vec<(f64, f64)> = Vec::new();
    
    let mut reading_nodes = false;
    let mut reading_sets = false;

    let mut profits = Vec::new();
    let mut nodes_of_cluster = Vec::new();
    let mut cluster_of_node = Vec::new();

    for line in reader.lines() {
        let line = line.expect("Error reading line");
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }

        if parts[0] == "DIMENSION:" {
            num_nodes = parts[1].parse().unwrap();
            cluster_of_node = vec![0; num_nodes];
        } else if parts[0] == "SETS:" {
            num_clusters = parts[1].parse().unwrap();
            profits = vec![0.0; num_clusters];
            nodes_of_cluster = vec![Vec::new(); num_clusters];
        } else if parts[0] == "TMAX:" {
            t_max = parts[1].parse().unwrap();
        } else if parts[0] == "NODE_COORD_SECTION" {
            reading_nodes = true;
            reading_sets = false;
        } else if parts[0] == "GTSP_SET_SECTION:" {
            reading_nodes = false;
            reading_sets = true;
        } else if parts[0] == "EOF" {
            break;
        } else if reading_nodes {
            let x: f64 = parts[1].parse().unwrap();
            let y: f64 = parts[2].parse().unwrap();
            coords.push((x, y));
        } else if reading_sets {
            let set_id: usize = parts[0].parse().unwrap();
            let profit: f64 = parts[1].parse().unwrap();
            profits[set_id] = profit;

            for i in 2..parts.len() {
                let node_id: usize = parts[i].parse().unwrap();
                let internal_node_id = node_id - 1; // Convert 1-indexed to 0-indexed
                nodes_of_cluster[set_id].push(internal_node_id);
                cluster_of_node[internal_node_id] = set_id;
            }
        }
    }

    // Precompute 2D Euclidean rounded UP (CEIL_2D from the instance specs)
    let mut dist = vec![0.0; num_nodes * num_nodes];
    for i in 0..num_nodes {
        for j in 0..num_nodes {
            if i != j {
                let dx = coords[i].0 - coords[j].0;
                let dy = coords[i].1 - coords[j].1;
                dist[i * num_nodes + j] = (dx * dx + dy * dy).sqrt().ceil();
            }
        }
    }

    Problem {
        num_nodes,
        num_clusters,
        t_max,
        dist,
        cluster_of_node,
        nodes_of_cluster,
        profits,
    }
}