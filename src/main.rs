mod benchmark;
mod catalog;
mod completion;
mod config;
mod context;
mod daemon;
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
use std::io::Read;
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
        #[arg(long, value_parser = clap::value_parser!(u16).range(6..=14))]
        height: Option<u16>,
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
        #[arg(long, value_parser = clap::value_parser!(u16).range(6..=14))]
        height: Option<u16>,
    },
    /// Socket Unix / motor persistente com cache em memória.
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Histórico local opt-in, guardando SOMENTE templates reconhecidos.
    Usage {
        #[command(subcommand)]
        action: UsageAction,
    },
    /// Configuração do popup, teclas e gatilhos.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
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
enum ConfigAction {
    /// Gera um config.toml com valores padrão.
    Init {
        #[arg(long)]
        force: bool,
    },
    /// Mostra configuração efetiva (TOML).
    Show,
    /// Exporta pares TSV seguros para o plugin Zsh.
    Zsh,
}

#[derive(Copy, Clone, Subcommand)]
enum DaemonAction {
    Start,
    Serve,
    Stop,
    Status,
    Reload,
    Socket,
    Bench,
}
impl DaemonAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Serve => "serve",
            Self::Stop => "stop",
            Self::Status => "status",
            Self::Reload => "reload",
            Self::Socket => "socket",
            Self::Bench => "bench",
        }
    }
}

#[derive(Subcommand)]
enum UsageAction {
    Record {
        #[arg(long, conflicts_with = "stdin")]
        line: Option<String>,
        #[arg(long)]
        stdin: bool,
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
    if let Commands::Config { action } = &cli.command {
        match action {
            ConfigAction::Init { force } => {
                println!("{}", config::Config::init(*force)?.display());
            }
            ConfigAction::Show => {
                println!("{}", toml::to_string_pretty(&config::Config::load()?)?);
            }
            ConfigAction::Zsh => {
                print!("{}", config::Config::load()?.zsh_pairs());
            }
        }
        return Ok(());
    }
    if let Commands::Daemon { action } = &cli.command {
        return daemon::command(action.as_str());
    }
    if let Commands::Inline { left } = &cli.command {
        if let Some(suffix) = daemon::inline(left) {
            if !suffix.is_empty() {
                println!("{suffix}");
            }
            return Ok(());
        }
    }
    if let Commands::Suggest {
        line,
        left,
        right,
        limit,
        format,
    } = &cli.command
    {
        let context = context(line.clone(), left.clone(), right.clone());
        if let Some(query) = context.query() {
            if let Some(matches) = daemon::suggest(query, *limit) {
                let items: Vec<ContextualSuggestion> = matches
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
                return Ok(());
            }
        }
    }
    let settings = config::Config::load()?;
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
            let mut popup = settings.popup.clone();
            if let Some(height) = height {
                popup.height = height;
            }
            if let Some(applied) = ui::pick(&catalog, &context, &usage, &popup)? {
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
            if !status && daemon::enabled() {
                let _ = daemon::query(&daemon::Request {
                    op: "reload".into(),
                    input: String::new(),
                    limit: 0,
                    path: String::new(),
                });
            }
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
                let mut style = settings.popup.clone();
                if let Some(height) = height {
                    style.height = height;
                }
                if let Some(name) = ui::recover_interactive(&word, &suggestions, &style)? {
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
            UsageAction::Record { line, stdin } => {
                if !Usage::enabled() {
                    return Err("Opt-in required: EITA_USAGE_ENABLED=1".into());
                }
                let input = if stdin {
                    let mut bytes = Vec::new();
                    std::io::stdin().take(4097).read_to_end(&mut bytes)?;
                    if bytes.len() > 4096 {
                        return Err("usage input too long".into());
                    }
                    String::from_utf8(bytes)?
                } else {
                    line.unwrap_or_default()
                };
                let mut stats = usage;
                if let Some(template) = stats.record(&input, &catalog) {
                    stats.save()?;
                    println!("{template}");
                    if daemon::enabled() {
                        let _ = daemon::query(&daemon::Request {
                            op: "reload".into(),
                            input: String::new(),
                            limit: 0,
                            path: String::new(),
                        });
                    }
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
        Commands::Config { .. } => unreachable!("config handled before catalog loading"),
        Commands::Daemon { .. } => unreachable!("daemon handled before catalog loading"),
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
