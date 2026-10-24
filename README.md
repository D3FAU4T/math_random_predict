# Math Random Predict

A high-performance Rust tool built to predict the output of JavaScript's `Math.random()` function across any runtime environment (Node.js, Bun, Safari, Chrome, browser contexts, etc.).

By utilizing a high-performance, pure-Rust symbolic bit-vector solver and CDCL SAT engine, this tool reconstructs the internal state of the PRNG engine used by the JavaScript runtime, allowing for accurate prediction of future generated values.

## Usage

A minimum of 3 outputs from `Math.random()` is required to reconstruct the internal state of the PRNG engine. The program can then predict the next output based on the reconstructed state.

### Terminal User Interface (TUI)

Just run the binary itself and follow the instructions in the terminal.

### Command-Line Interface (CLI)

The program also runs in CLI mode, allowing you to pass in a list of `Math.random()` outputs and receive the next predicted output.

```bash
./solver.exe <engine> <random_seed_1> <random_seed_2> <random_seed_3> ... <random_seed_n>
```

### Example

```bash
./solver.exe safari 0.6471651905689174 0.756716341278459 0.19666910380048563 0.18917313673581693 0.9714560740112332
```

> [!NOTE]
> Use `./solver.exe --help` to see the list of supported engines and their corresponding names.

## Prerequisites

To build this project from source, you only need:

- [Rust](https://www.rust-lang.org/tools/install) (2024 edition or newer)

No CMake, C/C++ compiler, or external solver installations are needed. The project is 100% pure Rust and fully cross-platform.

## Building

Build the project using Cargo:

```bash
cargo build --release
```

## Running Tests

Run the test suite using:

```bash
cargo test
```

# License

This project is licensed under the [MIT License](LICENSE).
