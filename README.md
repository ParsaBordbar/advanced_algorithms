# Set Orienteering Problem (SOP) - Metaheuristics in Rust

This repository contains highly optimized, pure Rust implementations of metaheuristic algorithms designed to solve the **Set Orienteering Problem (SOP)**. 

The focus of this project is on raw performance and algorithmic transparency. To achieve this, the implementations strictly adhere to sequential programming paradigms and use **zero external libraries** (relying solely on the Rust Standard Library).

## Directory Structure

The repository is divided into independent Rust Cargo projects, each implementing a specific metaheuristic approach:

- `sop_simulated_anealing/`: Solves the SOP using **Simulated Annealing (SA)**, mimicking the cooling process of metals to escape local optima.
- `sop_tabu/`: Solves the SOP using **Tabu Search (TS)**, utilizing memory structures (Tabu list) to avoid cycling and guide the search into unexplored regions.
- `sop_grasp/`: *(Upcoming)* Will solve the SOP using the **Greedy Randomized Adaptive Search Procedure (GRASP)**, combining greedy randomized construction with local search.

## The Set Orienteering Problem (SOP)

The SOP is a routing problem where the goal is to maximize the total collected profit within a given travel budget, denoted as $T_{max}$. 

Unlike the standard Orienteering Problem, vertices are grouped into **sets** (or clusters). Each set has an associated profit. To collect the profit of a set, a route must visit **at least one** vertex within that set. The route must start and end at predefined depot locations.

### Mathematical Objective
Maximize $\sum_{i \in V} P_i \cdot y_i$ 
Subject to: total route distance/cost $\le T_{max}$
*(Where $P_i$ is the profit of set $i$, and y_i is a binary variable indicating if set i was visited).*

## Design Choices & Representation

As speed and efficiency are the top priorities for this project, specific design choices were made across all implementations:

### 1. Pure Sequential Programming
The algorithms are entirely single-threaded and sequential. 
- **Why?** Avoiding parallelization overhead (like thread spawning, context switching, and synchronization locks) ensures that we are strictly measuring the algorithmic efficiency and convergence speed. It keeps the CPU cache dedicated to a single execution flow.

### 2. Zero External Dependencies
No external crates (not even popular ones like `rand` or `itertools`) are used.
- **Why?** To maintain absolute control over the execution flow and binary size. Any required components (such as a simple, highly efficient Pseudo-Random Number Generator like Xorshift or LCG) are implemented from scratch. This eliminates black-box overhead and ensures the codebase is self-contained.

### 3. Efficient Data Structures
Memory allocation in the main algorithmic loop is the enemy of speed. The project models the data to maximize cache locality:
- **Distance Matrix:** Represented as a flat 1D `Vec<f64>` instead of a `Vec<Vec<f64>>`. This ensures contiguous memory allocation, minimizing cache misses and allowing $O(1)$ distance lookups using index math: `index = row * width + col`.
- **Set & Vertex Representation:** Sets
