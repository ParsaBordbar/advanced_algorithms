# Tabu Search for the Set Orienteering Problem (SOP)

## 1. Introduction
This project implements a high-performance solver for the Set Orienteering Problem (SOP), based on the algorithmic framework described in the 2017 paper *"A tabu search algorithm for the set orienteering problem"*. The goal of SOP is to find a route that starts and ends at a specific depot, visits a subset of mutually exclusive clusters to collect profits, and respects a strict maximum travel distance ($T_{max}$). 

To maximize execution speed and strictly adhere to the constraint of using **no external libraries**, the solver is written entirely in standard Rust using strictly sequential programming paradigms. 

## 2. Problem Representation & Data Structures
To ensure cache-friendly data access and fast execution, the project is modularized and uses flat data structures.

*   **The `Problem` Struct:**
    Instead of using heavy object-oriented structures, the instance data is parsed into a flat, memory-contiguous struct. The distance matrix is flattened into a 1D `Vec<f64>` to optimize cache locality. A helper method computes the index dynamically: $index = i \times N + j$. The compiler is instructed to forcefully inline this method using `#[inline(always)]`.
*   **The `Solution` Struct:**
    A solution is represented by two parallel vectors: `tour_nodes` (the specific vertices visited) and `tour_clusters` (the clusters those vertices belong to). It caches `total_profit` and `total_cost` to prevent redundant $O(N)$ recalculations during local search.

## 3. Algorithmic Phases

### Phase 1: Constructive Heuristic
The solver begins with a greedy constructive heuristic. It iteratively evaluates unvisited clusters to insert into the tour. The evaluation metric is a ratio of the profit of the cluster to the insertion cost: 
$$Ratio = \frac{p_c}{\Delta C}$$
The heuristic stops when no further clusters can be added without violating the $T_{max}$ constraint.

### Phase 2: Local Search & Tour Improvement
Whenever the sequence of clusters changes, the route is optimized locally:
1.  **Vertex Selection (Dynamic Programming):** A DP algorithm runs to find the shortest path through the selected sequence of clusters.
2.  **2-Opt Routing:** Standard 2-Opt edge exchanges are applied to reduce the total travel distance, creating "slack" ($T_{max} - Cost$) which allows for the insertion of more clusters in the future.

### Phase 3: Tabu Search 
The main metaheuristic relies on a Tabu Search to escape local optima. 
*   **Neighborhoods:** The algorithm explores *Insert* (adding an unvisited cluster) and *Swap* (exchanging a visited cluster with an unvisited one) moves.
*   **Evaluation Function:** Moves are evaluated using a penalized objective function that allows temporary violations of $T_{max}$:
    $$F_{eval} = Profit - \lambda \times \max(0, Cost - T_{max})$$
*   **Diversification (SoftShake / HardShake):** Following the 2017 MASOP paper, if the algorithm is trapped without improvement, a destruction phase is applied. If the current profit is within $\beta = 8\%$ of the best known, a `SoftShake` removes $5\% - 15\%$ of the clusters. Otherwise, a `HardShake` removes $30\% - 40\%$.
*   **Tabu Lists & Tenure:** Recently added/removed clusters are placed in `tabu_insert` and `tabu_remove`. The Tabu tenure is dynamically generated.

## 4. Code Architecture: Modules and Functions
To maintain clean code organization and allow the Rust compiler to perform aggressive cross-module optimizations, the project is divided into specific components:

*   **`src/models.rs`:** Defines the `Problem` and `Solution` structures. Functions here include `Problem::get_dist` (for $O(1)$ flat-matrix lookups) and `Solution::recompute` (to recalculate costs/profits after a sequence perturbation).
*   **`src/parser.rs`:** Contains the parsing logic to read `.sop` files. It translates the string coordinates into the pre-computed Euclidean distance matrix. By calculating all distances at initialization, we save massive amounts of CPU cycles during the intensive local search phase.
*   **`src/rng.rs`:** Houses the custom Random Number Generators (`EpochRng` and `XorShift`). It exposes functions like `rand_tenure()` to quickly generate Tabu tenures without external dependencies.
*   **`src/solver/phase1.rs`:** 
    *   `initial_solution()`: Implements the greedy constructive approach.
    *   `tour_improvement()`: Implements the dynamic programming node selection and the 2-Opt neighborhood search.
*   **`src/solver/phase2.rs`:** The core Tabu Search engine.
    *   `tabu_search()`: The main loop tracking best solutions, managing Tabu lists, and calling neighborhoods.
    *   `explore_neighborhood()`: Iterates over all valid $O(N^2)$ Insert and Swap moves, rejecting Tabu moves unless they meet a global aspiration criteria.
    *   `apply_shake()`: Implements the Soft/Hard shake. Crucially, it uses an in-place array filter (`.retain()`) instead of a traditional loop with `.remove()`. This turns an $O(N^2)$ array shifting bottleneck into an ultra-fast $O(N)$ operation.
*   **`src/bin/benchmark.rs`:** A separate executable script designed to iterate over all instances in a folder and output CSV metrics for performance tracking.

## 5. Custom PRNG Implementation
Since external crates (like `rand`) were forbidden, a custom Pseudo-Random Number Generator (`EpochRng` / `XorShift`) was implemented. It is seeded using `SystemTime::now().duration_since(UNIX_EPOCH)`. Internally, it uses a 64-bit Linear Congruential Generator (LCG) or bit-shift mechanisms. This guarantees extremely fast random number generation for the Tabu tenure without system call overhead.

## 6. Performance Optimizations & Execution Speed
*   **Zero Dependencies:** The standard library `std` is the sole dependency.
*   **Cross-Module LTO:** Compiling with `cargo build --release` allows Rust's LLVM backend to perform Cross-Language Time Optimization (LTO) and unroll loops, achieving near-C performance.
*   **Memory Allocations:** `Vec` allocations are kept outside tight inner loops wherever possible. When nodes must be removed from a vector (e.g., during a Shake phase), the `.retain()` method is used. This filters the list in-place rather than shifting elements repeatedly, preventing severe memory bottlenecking.

## 7. Benchmark Results
The algorithm was benchmarked across various instances. The execution times (measured in milliseconds) show that the algorithm is highly efficient, consistently solving mid-to-large-scale instances in under a few seconds without relying on parallel threads.

| Instance | Profit | Cost | Time (ms) | | Instance | Profit | Cost | Time (ms) |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `instance_2.sop` | 4550 | 5102 | 82 | | `instance_82.sop` | 10867 | 2524 | 318 |
| `instance_6.sop` | 9338 | 10602 | 129 | | `instance_86.sop` | 18495 | 5061 | 958 |
| `instance_8.sop` | 9970 | 12717 | 159 | | `instance_88.sop` | 20028 | 6355 | 1632 |
| `instance_11.sop` | 138 | 7859 | 70 | | `instance_91.sop` | 260 | 5780 | 458 |
| `instance_14.sop` | 8892 | 10486 | 121 | | `instance_95.sop` | 415 | 9345 | 1855 |
| `instance_16.sop` | 9990 | 12975 | 184 | | `instance_99.sop` | 378 | 35808 | 752 |
| `instance_19.sop` | 151 | 40608 | 134 | | `instance_104.sop` | 21976 | 60090 | 1337 |
| `instance_27.sop` | 139 | 964 | 112 | | `instance_107.sop` | 315 | 12873 | 743 |
| `instance_31.sop` | 219 | 1600 | 180 | | `instance_109.sop` | 391 | 17288 | 1370 |
| `instance_33.sop` | 84 | 25422 | 72 | | `instance_113.sop` | 278 | 7994 | 407 |
| `instance_40.sop` | 11368 | 63207 | 52 | | `instance_118.sop` | 23667 | 15987 | 2128 |
| `instance_41.sop` | 104 | 401 | 70 | | `instance_121.sop` | 262 | 955 | 1006 |
| `instance_45.sop` | 225 | 802 | 237 | | `instance_125.sop` | 491 | 1903 | 5866 |
| `instance_47.sop` | 258 | 1012 | 549 | | `instance_128.sop` | 28434 | 2376 | 9281 |
| `instance_50.sop` | 6382 | 11262 | 121 | | `instance_131.sop` | 442 | 9993 | 3411 |
| `instance_53.sop` | 231 | 23533 | 178 | | `instance_132.sop` | 22499 | 10013 | 2869 |
| `instance_58.sop` | 5580 | 429 | 122 | | `instance_133.sop` | 539 | 13348 | 5078 |
| `instance_64.sop` | 13971 | 1072 | 897 | | `instance_137.sop` | 168 | 10962 | 233 |
| `instance_66.sop` | 6600 | 8980 | 225 | | `instance_141.sop` | 634 | 21829 | 6994 |
| `instance_67.sop` | 187 | 13553 | 162 | | `instance_142.sop` | 32309 | 21933 | 4673 |
| `instance_71.sop` | 296 | 22384 | 467 | | `instance_144.sop` | 32985 | 27338 | 8165 |
| `instance_77.sop` | 295 | 16591 | 900 | | | | | |

***

## Appendix: Special Rust Syntax & Concepts Used
Because Rust focuses heavily on memory safety without a garbage collector, several unique syntactical elements are utilized in this codebase to guarantee speed:

1.  **Ownership and Borrowing (`&` and `&mut`):** 
    Instead of passing objects by value (which duplicates them in memory), we use references. A read-only reference `&Problem` ensures the massive distance matrix isn't duplicated, while `&mut current_sol` gives a function exclusive, safe rights to modify the route. This guarantees there are no data races or hidden performance penalties.
2.  **Slices (`&[usize]`):**
    When interacting with arrays (like the Tabu lists), we pass them as slices (e.g., `tabu_insert: &[usize]`). This allows a function to view contiguous memory dynamically without needing to know its exact size at compile-time, ensuring safe bounds checking.
3.  **Compiler Directives (`#[inline(always)]`):**
    By placing this tag above small, highly frequent functions (like `get_dist`), we instruct the Rust compiler to physically copy the function's bytecode wherever it is called. This eliminates the CPU overhead of a "function call jump" during millions of Tabu iterations.
4.  **`Option<T>` Enum:**
    Rust has no concept of `null`. To represent a state where a "best neighbor" might not have been found yet, we use `Option<Solution>`. It forces the programmer to explicitly handle both `Some(solution)` and `None` branches via pattern matching (e.g., `if let Some(cand) = neighbor { ... }`). This completely eliminates Null Pointer Exceptions.
5.  **Closures (`|args| -> ret_type { body }`):**
    In `explore_neighborhood`, an inline anonymous function (closure) is defined to evaluate solutions: `let evaluate = |cand, best| -> bool { ... };`. This keeps the tie-breaking logic modular and readable without having to pass large chunks of state to a distant, separate function.
6.  **In-Place Retain (`.retain(|_| { ... })`):**
    During the Shake phase, instead of looping over the solution vector and calling `.remove(index)`—which forces all subsequent elements in the array to physically shift their memory addresses ($O(N)$ overhead per removal)—we use `.retain()`. This internal Rust optimization evaluates a true/false condition for every element and packs the memory buffer tightly in a single $O(N)$ pass.