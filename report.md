<div align="center">
  <h2>The Set Orienteering Problem</h2>
  <h3>(SOP) With SA</h3>
  <p><strong>Parsa Bordbar 40435340</strong></p>
  <p>Dr. K.Zearati</p>
  <p>April 11, 2026 | 1405/01/22</p>
</div>

***

## 1. Representation & Data Structures

To achieve maximum execution speed with zero reliance on external libraries, the architecture utilizes contiguous memory blocks, sequential operations, and cache-friendly data structures.

**The Problem Struct**
The `Problem` struct holds the core components of the parsed instance. To bypass the memory fragmentation and pointer-chasing overhead of nested vectors (e.g., `Vec<Vec<f64>>`), the distance matrix is flattened into a 1D array (`dist`). Finding the distance between node $i$ and node $j$ is strictly an $\mathcal{O}(1)$ operation using the formula $index = i \times num\_nodes + j$. The struct also stores cluster memberships, individual cluster profits, and the strict upper limit for travel time ($T_{max}$).

**The Solution Struct**
The `Solution` struct represents a routing state. Rather than dynamically recomputing sequences from scratch, it maintains two sequential vectors: `tour_nodes` and `tour_clusters`. It also caches the $total\_cost$ and $total\_profit$. During search iterations, we do not perform full recalculations of the $\mathcal{O}(N)$ route. Instead, we compute localized $\mathcal{O}(1)$ deltas for proposed moves and apply the differences to the cached totals.

---

## 2. Algorithmic Phases

### Phase 1.1: Constructive Heuristic
A robust initial solution provides the metaheuristic with a strong starting boundary. The initial construction (`solver/phase1.rs`) employs a greedy strategy:
1. Clusters are sorted in descending order of their available profit.
2. For each cluster, the algorithm selects the most promising node that guarantees the minimum addition to the total travel cost.
3. A cluster is only inserted if the resulting $total\_cost$ remains strictly $\le T_{max}$.

### Phase 1.2: Local Search & Tour Improvement
Once a base sequence is constructed, it is subjected to immediate tour improvement techniques before Simulated Annealing begins:
1. **Vertex Selection (Dynamic Programming):** Given a fixed sequence of clusters, a DP algorithm calculates the optimal exact sequence of nodes that minimizes the total travel cost.
2. **2-Opt Routing:** A 2-opt edge-exchange mechanism untangles crossing edges. Reversing a segment in `tour_nodes` and its corresponding segment in `tour_clusters` significantly optimizes spatial routing.

### Phase 2: Simulated Annealing
The core optimization phase relies on a heavily tuned Simulated Annealing (SA) algorithm designed to mirror the findings of the recent literature on SOP. 

*   **Neighborhoods:**
    The search explores an expanded geometry uniformly selecting from multiple moves:
    *   **Insertion:** Moving a node/cluster to a different point in the sequence.
    *   **Drop:** Removing a cluster to free up capacity.
    *   **Replace:** Swapping a cluster inside the route with an unvisited cluster outside.
    *   **Swap (15%):** Exchanging two positions $i$ and $j$ inside the tour.
    *   **Inversion / 2-Opt (20%):** Reversing a sub-segment `[p1..=p2]`.
    *   **Node Change (15%):** Swapping an active node for a different node belonging to the same cluster.
    All evaluations are done using $\mathcal{O}(1)$ delta math to evaluate millions of states per second.

*   **Evaluation Function & Penalties:**
    The objective dynamically handles constraints. Instead of discarding infeasible moves, they are permitted but heavily penalized. The evaluation applies a penalty factor of $100.0$: 
    $$ \text{Objective} = \text{Profit} - 100.0 \times \max(0, \text{Cost} - T_{max}) $$

*   **Cooling Schedule & Parameters:**
    Following tuned best-practices, the hyperparameters are fixed to:
    *   $T_0 = 16.0$ (Initial Temperature)
    *   $\alpha = 0.93$ (Geometric Cooling rate: $T_{k+1} = \alpha \cdot T_k$)
    *   Epoch Length: $80,000$ iterations per temperature level.
    *   Acceptance uses standard Metropolis criteria: if $\Delta \ge 0$, accept. If $\Delta < 0$, accept with probability $P = \exp(\Delta / T)$.

*   **Initialization, Intensification & Diversification:**
    High initial temperatures combined with large epochs (80,000) heavily diversify the search early on. Because localized $\mathcal{O}(1)$ delta calculations accumulate floating-point drift over millions of steps, the system implements a cyclic $\mathcal{O}(N)$ `recompute()` every $100,000$ iterations to guarantee objective purity. 

---

## 3. Code Architecture: Modules and Functions

The codebase (`sop_simulated_annealing/src/`) strictly modularizes responsibilities:
*   `main.rs` & `benchmark/`: Entry points for single-instance parsing and multi-instance batch benchmarking. Handles CLI arguments and overall execution flow.
*   `models.rs`: Defines the `Problem` and `Solution` properties, along with their high-speed implementation methods.
*   `io/parser.rs` & `io/output.rs`: Reads `.sop` files. Converts 1-indexed to 0-indexed values. Euclidean distances are computed with ceiling applied (`CEIL_2D`) to ensure integer-like strictness in floats.
*   `feasibility.rs`: Acts as an unyielding validator. It ensures the tour begins/ends at the depot, checks lengths, confirms node-cluster legitimacy, and enforces the $T_{max}$ strict limit.
*   `solver/`: 
    *   `init.rs` / `phase1.rs`: Constructive heuristic generators.
    *   `simulated_annealing.rs`: The heartbeat of the engine housing the SA loops, cooling schedule, and randomized neighborhood generators.
*   `rng.rs`: A deterministic PRNG module to keep the library dependency-free.

---

## 4. Custom PRNG Implementation

Because external dependencies (like the `rand` crate) are forbidden for this project's constraints, `rng.rs` contains a custom, lightweight Pseudo-Random Number Generator. Using a linear congruential generator (LCG) or Xorshift structure, this inline PRNG is blazingly fast. It serves integer distributions for choosing neighbor indices and uniform $[0, 1)$ float distributions necessary for the Metropolis acceptance probability $P = \exp(\Delta / T)$.

---

## 5. Performance Optimizations & Execution Speed

Execution speed dictates the extent of search depth. To achieve execution times of ~130ms while evaluating massive epoch lengths:
*   **Zero Dynamic Allocation:** Inside the main annealing loop, no new `Vec`s are allocated. Route changes are performed via in-place mutation.
*   **$\mathcal{O}(1)$ Edge Reconnection:** Determining the cost change of a SWAP or INVERSION requires analyzing only the $2$ to $4$ edges being broken and formed, rather than looping over the $N$-length arrays.
*   **Memory Footprint:** Sequential vector alignment ensures that memory blocks are prefetched into the CPU cache (L1/L2), drastically reducing cycle stalls.

---

## 6. Benchmark Results

The SA framework successfully tackles complex instances, routinely finishing evaluations in just over a tenth of a second while achieving strong BKS approximations.

| Instance | Best Profit | Best Cost | Avg Time (ms) |
| :--- | :--- | :--- | :--- |
| instance_2.sop | 3608 | 5288 | 122 |
| instance_6.sop | 9115 | 10266 | 131 |
| instance_11.sop | 131 | 7783 | 138 |
| instance_19.sop | 148 | 40030 | 139 |
| instance_40.sop | 11347 | 61434 | 129 |
| instance_64.sop | 13429 | 1034 | 137 |
| instance_86.sop | 17366 | 4957 | 135 |
| instance_104.sop | 21174 | 51852 | 142 |
| instance_128.sop | 26944 | 2308 | 137 |
| instance_144.sop | 32735 | 24016 | 137 |

*(Note: Extracted sample. All 43 benchmarked instances ran between 122ms and 149ms, showcasing an incredibly tight variance in execution duration regardless of instance scale).*

---

## 7. Appendix: Special Rust Syntax & Concepts

To enforce our strict speed and memory parameters, several Rust-specific paradigms are heavily utilized:
*   **Ownership and Borrowing:** By leveraging mutable references (`&mut Solution`), the codebase guarantees memory safety without the runtime overhead of a Garbage Collector.
*   **Slices:** `&[T]` slices are used for reading segments of routes during 2-Opt inversions, avoiding costly cloning operations.
*   **Inline Functions:** Mathematical calculations and custom PRNG state updates are tagged with `#[inline(always)]`, instructing the compiler to inject the assembly directly into the call site to eliminate stack-frame overhead.
*   **Option<T> & Result Types:** Used aggressively during initial node selections to safely prevent null-pointer exceptions without sacrificing runtime speed.
