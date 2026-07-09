mod browser;
mod safari;
mod v8;

use clap::{Parser, ValueEnum};
use safari::SafariPredictor;
use v8::{V8Predictor, V8Mode};
use browser::BrowserPredictor;
use std::io::{self, Write};

#[derive(Parser, Debug)]
#[command(name = "solver", about = "Math.random() Predictor - State Solver & Number Predictor")]
struct Cli {
    #[arg(value_enum)]
    engine: Option<Engine>,

    #[arg(num_args = 1..)]
    numbers: Vec<String>,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Engine {
    #[value(name = "v8-modern")]
    V8Modern,
    #[value(name = "v8-v24")]
    V8V24,
    #[value(name = "v8-legacy")]
    V8Legacy,
    #[value(name = "safari")]
    Safari,
    #[value(name = "browser")]
    Browser,
}

fn parse_and_sanitize_input(input: &str) -> Vec<f64> {
    let sanitized = input
        .replace('[', " ")
        .replace(']', " ")
        .replace('(', " ")
        .replace(')', " ")
        .replace(',', " ")
        .replace(';', " ")
        .replace('"', " ")
        .replace('\'', " ");

    sanitized
        .split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let cli = Cli::parse();

        let engine = match cli.engine {
            Some(e) => e,
            None => {
                eprintln!("Error: Engine is required as the first argument.");
                std::process::exit(1);
            }
        };

        let raw_input = cli.numbers.join(" ");
        let numbers = parse_and_sanitize_input(&raw_input);

        if numbers.len() < 3 {
            eprintln!("Error: You must provide at least 3 numbers.");
            std::process::exit(1);
        }

        run_predictor(engine, numbers);
    } else {
        interactive_menu();
    }
}

fn interactive_menu() {
    loop {
        println!("\n=== Math.random() Predictor ===");
        println!("Select the target engine:");
        println!("  1) V8 Modern (Node.js >= 26)");
        println!("  2) V8 v24/25 (Node.js 24, 25)");
        println!("  3) V8 Legacy (Node.js < 24)");
        println!("  4) Safari (Safari, Bun, JavaScriptCore)");
        println!("  5) Browser (Chrome, Edge, Firefox, SpiderMonkey)");
        println!("  6) Exit");

        print!("Enter choice (1-6): ");
        io::stdout().flush().unwrap();
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        let choice = choice.trim();

        if choice == "6" {
            break;
        }

        let engine = match choice {
            "1" => Engine::V8Modern,
            "2" => Engine::V8V24,
            "3" => Engine::V8Legacy,
            "4" => Engine::Safari,
            "5" => Engine::Browser,
            _ => {
                println!("Invalid selection.");
                continue;
            }
        };

        loop {
            print!("\nEnter space or comma-separated random numbers (at least 3): ");
            io::stdout().flush().unwrap();
            let mut num_input = String::new();
            io::stdin().read_line(&mut num_input).unwrap();
            let numbers = parse_and_sanitize_input(&num_input);

            if numbers.len() < 3 {
                println!("Error: You must provide at least 3 numbers. Got: {}", numbers.len());
                continue;
            }

            println!("\nPredicted value:");
            if let Some(prediction) = run_predictor_opt(engine, numbers) {
                println!("{}", prediction);
            } else {
                println!("Error: Could not solve the state using the provided sequence.");
            }
            break;
        }

        loop {
            print!("\nWhat would you like to do?\n  1) Go back to Main Menu\n  2) Exit\nEnter choice (1-2): ");
            io::stdout().flush().unwrap();
            let mut post_choice = String::new();
            io::stdin().read_line(&mut post_choice).unwrap();
            let post_choice = post_choice.trim();
            if post_choice == "2" {
                return;
            } else if post_choice == "1" {
                break;
            } else {
                println!("Invalid selection.");
            }
        }
    }
}

fn run_predictor_opt(engine: Engine, numbers: Vec<f64>) -> Option<f64> {
    match engine {
        Engine::Safari => {
            let mut predictor = SafariPredictor::new(numbers);
            if predictor.solve() { Some(predictor.predict_next()) } else { None }
        }
        Engine::Browser => {
            let mut predictor = BrowserPredictor::new(numbers);
            if predictor.solve() { Some(predictor.predict_next()) } else { None }
        }
        Engine::V8Modern => {
            let mut predictor = V8Predictor::new(numbers, V8Mode::Modern);
            if predictor.solve() { Some(predictor.predict_next()) } else { None }
        }
        Engine::V8V24 => {
            let mut predictor = V8Predictor::new(numbers, V8Mode::V24);
            if predictor.solve() { Some(predictor.predict_next()) } else { None }
        }
        Engine::V8Legacy => {
            let mut predictor = V8Predictor::new(numbers, V8Mode::Legacy);
            if predictor.solve() { Some(predictor.predict_next()) } else { None }
        }
    }
}

fn run_predictor(engine: Engine, numbers: Vec<f64>) {
    if let Some(prediction) = run_predictor_opt(engine, numbers) {
        println!("{}", prediction);
    } else {
        eprintln!("Error: Could not solve the state using the provided sequence.");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v8_modern_prediction() {
        let seq = vec![
            0.28117848923781774, 0.9070931958847521,
            0.6021825103171052, 0.10512728754914236,
            0.5323637996484628,
        ];
        let mut predictor = V8Predictor::new(seq, V8Mode::Modern);
        assert!(predictor.solve());
        assert!((predictor.predict_next() - 0.24988691361986748).abs() < 1e-15);
    }

    #[test]
    fn test_browser_prediction() {
        let seq = vec![
            0.25689179146029395, 0.6158186928407353,
            0.8060743499760569, 0.30338630244882003,
            0.7374330371665526,
        ];
        let mut predictor = BrowserPredictor::new(seq);
        assert!(predictor.solve());
        assert!((predictor.predict_next() - 0.16600059674118928).abs() < 1e-15);
    }

    #[test]
    fn test_input_sanitizer() {
        let res = parse_and_sanitize_input("[0.123, 0.456, 0.789]");
        assert_eq!(res, vec![0.123, 0.456, 0.789]);
    }

    #[test]
    fn test_minimum_3_inputs() {
        let seq = vec![
            0.28117848923781774, 0.9070931958847521,
            0.6021825103171052,
        ];
        let mut predictor = V8Predictor::new(seq, V8Mode::Modern);
        assert!(predictor.solve());
        assert!((predictor.predict_next() - 0.10512728754914236).abs() < 1e-15);
    }
}
