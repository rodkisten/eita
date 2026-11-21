mod catalog;
mod engine;
mod specs;
mod ui;

use clap::{Parser, Subcommand, ValueEnum};
use std::error::Error;
use std::fs;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "eita",
    version,
    about = "Sugestoes contextuais de comandos para Zsh"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Retorna sugestoes sem abrir interface.
    Suggest {
        #[arg(long)]
        line: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long, value_enum, default_value_t = Format::Json)]
        format: Format,
    },
    /// Abre seletor Ratatui e grava a escolha em um arquivo.
    Pick {
        #[arg(long)]
        line: String,
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// Mede o tempo de matching do catalogo ja carregado.
    Bench {
        #[arg(long, default_value_t = 200)]
        iterations: usize,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Format {
    Json,
    Lines,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let catalog = catalog::load_catalog();
    let command_specs = specs::load();
    match cli.command {
        Commands::Suggest {
            line,
            limit,
            format,
        } => {
            let suggestions = engine::suggest_with_specs(&catalog, &command_specs, &line, limit);
            match format {
                Format::Json => println!("{}", serde_json::to_string_pretty(&suggestions)?),
                Format::Lines => {
                    for suggestion in suggestions {
                        println!("{}", suggestion.value);
                    }
                }
            }
        }
        Commands::Pick { line, output } => {
            if let Some(value) = ui::pick(&catalog, &command_specs, &line)? {
                if let Some(path) = output {
                    fs::write(path, value.as_bytes())?;
                } else {
                    println!("{value}");
                }
            }
        }
        Commands::Bench { iterations } => {
            if iterations == 0 {
                return Err("--iterations deve ser maior que zero".into());
            }
            let samples = [
                "git comit --am",
                "docker comp",
                "pnpm i",
                "got",
                "gh pr cre",
            ];
            let mut latencies = Vec::with_capacity(iterations);
            for index in 0..iterations {
                let start = Instant::now();
                std::hint::black_box(engine::suggest(
                    &catalog,
                    samples[index % samples.len()],
                    10,
                ));
                latencies.push(start.elapsed().as_micros());
            }
            latencies.sort_unstable();
            let percentile = |n: usize| latencies[(latencies.len() - 1) * n / 100];
            println!("candidates={} iterations={}", catalog.len(), iterations);
            println!(
                "p50={}us p95={}us p99={}us",
                percentile(50),
                percentile(95),
                percentile(99)
            );
            println!("Search only; excludes startup, indexing and terminal rendering.");
        }
    }
    Ok(())
}
