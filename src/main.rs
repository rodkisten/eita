mod benchmark;
mod catalog;
mod completion;
mod context;
mod engine;
mod inline;
mod recover;
mod ui;
mod usage;

use clap::{Parser, Subcommand, ValueEnum};
use context::Context;
use serde::Serialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use usage::Usage;

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
        /// Linha inteira (compatibilidade com v0.1).
        #[arg(long, conflicts_with = "left")]
        line: Option<String>,
        /// Texto antes do cursor (ZLE LBUFFER).
        #[arg(long)]
        left: Option<String>,
        /// Texto depois do cursor (ZLE RBUFFER).
        #[arg(long, default_value = "")]
        right: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(long, value_enum, default_value_t = Format::Json)]
        format: Format,
    },
    /// Abre o seletor Ratatui (também acessível como 'popup').
    #[command(alias = "popup")]
    Pick {
        #[arg(long, conflicts_with = "left")]
        line: Option<String>,
        #[arg(long)]
        left: Option<String>,
        #[arg(long, default_value = "")]
        right: String,
        #[arg(long)]
        output: Option<PathBuf>,
        /// Cursor em caracteres Unicode, nao bytes.
        #[arg(long, requires = "output")]
        cursor_output: Option<PathBuf>,
        /// Altura compacta para terminais móveis (6 a 12 linhas).
        #[arg(long, default_value_t = 8, value_parser = clap::value_parser!(u16).range(6..=12))]
        height: u16,
    },
    /// Indexa definições literais das completions nativas do Zsh sem executá-las.
    Index {
        /// Força a reconstrução do índice (padrão quando não usa --status).
        #[arg(long)]
        refresh: bool,
        /// Lista de diretórios do fpath separada por dois-pontos.
        #[arg(long)]
        fpath: Option<String>,
        /// Substitui o caminho do cache usado nesta operação.
        #[arg(long)]
        cache: Option<PathBuf>,
        /// Exibe estatísticas do cache sem reconstruir o índice.
        #[arg(long, conflicts_with = "refresh")]
        status: bool,
    },
    /// Exibe apenas o sufixo seguro para POSTDISPLAY no ZLE.
    Inline {
        #[arg(long)]
        left: String,
    },
    /// Corrige erros usando simbolos reais (comandos, aliases, funcoes ou variaveis).
    Recover {
        #[arg(long)]
        word: String,
        #[arg(long)]
        symbols_file: PathBuf,
        #[arg(long, value_enum, default_value_t = RecoveryKind::Command)]
        kind: RecoveryKind,
        #[arg(long, default_value_t = 5)]
        limit: usize,
        #[arg(long, value_enum, default_value_t = Format::Json)]
        format: Format,
        #[arg(long)]
        popup: bool,
        #[arg(long, requires = "popup")]
        output: Option<PathBuf>,
        #[arg(long, default_value_t = 8, value_parser = clap::value_parser!(u16).range(6..=12))]
        height: u16,
    },
    /// Histórico local opt-in, guardando SOMENTE templates reconhecidos.
    Usage {
        #[command(subcommand)]
        action: UsageAction,
    },
    /// Mede busca ou compara Frizbee e Nucleo no mesmo dataset.
    Bench {
        #[arg(long, default_value_t = 200)]
        iterations: usize,
        #[arg(long)]
        compare: bool,
    },
}

#[derive(Subcommand)]
enum UsageAction {
    Record {
        #[arg(long)]
        line: String,
    },
    Status,
    Clear,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum RecoveryKind {
    Command,
    Alias,
    Function,
    Builtin,
    Reserved,
    Variable,
    All,
}

impl RecoveryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Alias => "alias",
            Self::Function => "function",
            Self::Builtin => "builtin",
            Self::Reserved => "reserved",
            Self::Variable => "variable",
            Self::All => "all",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Format {
    Json,
    Lines,
}

#[derive(Serialize)]
struct ContextualSuggestion {
    #[serde(flatten)]
    suggestion: engine::Suggestion,
    replacement: String,
    cursor: usize,
}

fn context(line: Option<String>, left: Option<String>, right: String) -> Context {
    let preceding = left.or(line).unwrap_or_default();
    Context::new(&preceding, &right)
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let catalog = if matches!(
        &cli.command,
        Commands::Inline { .. } | Commands::Recover { .. }
    ) {
        catalog::load_inline_catalog()
    } else {
        catalog::load_catalog()
    };
    let usage = Usage::load();
    match cli.command {
        Commands::Suggest {
            line,
            left,
            right,
            limit,
            format,
        } => {
            let context = context(line, left, right);
            let items: Vec<ContextualSuggestion> = context
                .query()
                .map(|query| engine::suggest_ranked(&catalog, query, limit, &usage))
                .unwrap_or_default()
                .into_iter()
                .map(|suggestion| {
                    let applied = context.apply(&suggestion.value);
                    ContextualSuggestion {
                        suggestion,
                        replacement: applied.replacement,
                        cursor: applied.cursor,
                    }
                })
                .collect();
            match format {
                Format::Json => println!("{}", serde_json::to_string_pretty(&items)?),
                Format::Lines => {
                    for item in items {
                        println!("{}", item.replacement);
                    }
                }
            }
        }
        Commands::Pick {
            line,
            left,
            right,
            output,
            cursor_output,
            height,
        } => {
            let context = context(line, left, right);
            if let Some(applied) = ui::pick(&catalog, &context, &usage, height)? {
                if let Some(path) = output {
                    fs::write(path, applied.replacement.as_bytes())?;
                    if let Some(cursor_path) = cursor_output {
                        fs::write(cursor_path, applied.cursor.to_string())?;
                    }
                } else {
                    println!("{}", applied.replacement);
                }
            }
        }
        Commands::Index {
            refresh: _,
            fpath,
            cache,
            status,
        } => {
            let destination = cache.unwrap_or_else(completion::cache_path);
            let report = if status {
                completion::status(&destination)?
            } else {
                completion::rebuild(fpath.as_deref(), &destination)?
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Commands::Recover {
            word,
            symbols_file,
            kind,
            limit,
            format,
            popup,
            output,
            height,
        } => {
            let symbols = recover::read_symbols(&symbols_file)?;
            let suggestions = recover::recover(&word, &symbols, kind.as_str(), limit);
            if popup {
                if let Some(name) = ui::recover_interactive(&word, &suggestions, height)? {
                    if let Some(path) = output {
                        fs::write(path, name.as_bytes())?;
                    } else {
                        println!("{name}");
                    }
                }
            } else {
                match format {
                    Format::Json => {
                        println!("{}", serde_json::to_string_pretty(&suggestions)?);
                    }
                    Format::Lines => {
                        for candidate in suggestions {
                            println!("{}", candidate.name);
                        }
                    }
                }
            }
        }
        Commands::Inline { left } => {
            if let Some(text) = inline::suffix(&catalog, &left, &usage) {
                println!("{text}");
            }
        }
        Commands::Usage { action } => match action {
            UsageAction::Record { line } => {
                if !Usage::enabled() {
                    return Err("Opt-in required: EITA_USAGE_ENABLED=1".into());
                }
                let mut stats = usage;
                if let Some(template) = stats.record(&line, &catalog) {
                    stats.save()?;
                    println!("{template}");
                }
            }
            UsageAction::Status => {
                println!("{}", serde_json::to_string_pretty(&usage.summary())?);
            }
            UsageAction::Clear => {
                Usage::clear()?;
                println!("Local template usage cleared");
            }
        },
        Commands::Bench {
            iterations,
            compare,
        } => {
            if compare {
                println!("{}", benchmark::compare(&catalog, iterations)?);
                return Ok(());
            }
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
