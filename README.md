# Contagent Bevy

> **⚠️ WARNING:** This is not production-ready code. For the stable, production-ready version of this simulation, please use the Kotlin implementation at [github.com/0xr0bert/contagent](https://github.com/0xr0bert/contagent) for now.

A high-performance, deterministic agent-based simulation built with the Bevy engine in Rust. This project simulates agent belief perception and action selection based on complex relationships and performance metrics.

## Features

- **Parallel Execution:** Leverages Bevy's ECS and Rayon for multi-core simulation performance.
- **Deterministic Simulation:** Supports reproducible runs through command-line RNG seeding.
- **Zstandard Compression:** Handles large agent datasets efficiently using Zstd-compressed JSON.
- **Flexible Output:** Supports exporting either full agent states or statistical summaries (mean, SD, median, etc.).
- **Robust UUID Linking:** Automatically resolves cross-entity relationships (friends, perceptions, actions) using stable UUIDs.

## Installation

Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed.

```bash
git clone https://github.com/your-repo/contagent-bevy
cd contagent-bevy
cargo build --release
```

## Usage

The application requires several positional arguments to define the simulation parameters and data paths.

```bash
cargo run -- <start-tick> <end-tick> <agents-path> <beliefs-path> <behaviours-path> <output-path> <full-output-bool> [seed]
```

### Arguments

1.  `start-tick`: The tick at which the simulation begins (e.g., `1`).
2.  `end-tick`: The tick at which the simulation ends (e.g., `1000`).
3.  `agents-path`: Path to the Zstd-compressed agents JSON file (`.json.zst`).
4.  `beliefs-path`: Path to the beliefs JSON file.
5.  `behaviours-path`: Path to the behaviours JSON file.
6.  `output-path`: Path where the output will be saved (`.json.zst`).
7.  `full-output-bool`: `true` to export full agent state history, `false` for statistical summaries.
8.  `seed` (Optional): A 64-bit integer to seed the RNG for deterministic results.

### Example

Run a 100-tick simulation using a specific seed and outputting summaries:
```bash
cargo run -- 1 100 agents.json.zst beliefs.json behaviours.json summary.json.zst false 123456789
```

## Data Schema

### Behaviours (`behaviours.json`)
```json
[
  { "name": "Walk", "uuid": "..." },
  { "name": "Drive", "uuid": "..." }
]
```

### Beliefs (`beliefs.json`)
```json
[
  {
    "uuid": "...",
    "name": "Trust",
    "relationships": { "TARGET_UUID": 0.5 },
    "perceptions": { "TARGET_UUID": 0.2 }
  }
]
```

### Agents (`agents.json.zst`)
The agents file is a Zstd-compressed JSON array of objects following this structure:
- `uuid`: String
- `actions`: List of behavior UUIDs (historical).
- `activations`: List of belief activation maps.
- `deltas`: Map of belief decay/growth rates.
- `friends`: Map of friend weights.
- `performance_relationships`: Nested map of belief-to-behavior performance weights.

## License

This project is licensed under the BSD 3-Clause License. See the [LICENSE](LICENSE) file for details.
