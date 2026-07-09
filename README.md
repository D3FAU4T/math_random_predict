# Math Random Predict

A high-performance Rust tool built to predict the output of JavaScript's `Math.random()` function across any runtime environment (Node.js, Bun, Safari, Chrome, browser contexts, etc.). 

By utilizing the power of the Z3 Theorem Prover, this tool reconstructs the internal state of the PRNG engine used by the JavaScript runtime, allowing for accurate prediction of future generated values.

## Usage

A minimum of 3 outputs from `Math.random()` is required to reconstruct the internal state of the PRNG engine. The program can then predict the next output based on the reconstructed state.

### Terminal User Interface (TUI)

Just run the binary itself and follow the instructions in the terminal.

### Command-Line Interface (CLI)

The program also runs in CLI mode, allowing you to pass in a list of `Math.random()` outputs and receive the next predicted output.

```bash
./solver.exe <engine> <random_seed_1> <random_seed_2> <random_seed_3> ... <random_seed_n>
```

Note: Use `./solver.exe --help` to see the list of supported engines and their corresponding names.

## Prerequisites

To build this project from source, you will need:

- [Rust](https://www.rust-lang.org/tools/install)
- CMake
- A C++ toolchain (Visual Studio Build Tools 2022 is expected on Windows, as configured in `.cargo/config.toml`).

## Building

Build the project using Cargo. Note that the initial compilation will take a few minutes (around 5 minutes depending on hardware) because it compiles the entire Z3 C++ codebase from scratch to optimize execution speed.

```bash
cargo build --release
```

# License

This project is licensed under the [MIT License](LICENSE).