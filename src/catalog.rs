use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::time::UNIX_EPOCH;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Candidate {
    pub value: String,
    pub description: String,
    pub source: String,
}

impl Candidate {
    fn new(value: &str, description: &str, source: &str) -> Self {
        Self {
            value: value.to_owned(),
            description: description.to_owned(),
            source: source.to_owned(),
        }
    }

    pub fn root(&self) -> &str {
        self.value.split_whitespace().next().unwrap_or("")
    }
}

const BUILTINS: &[(&str, &str)] = &[
    ("git", "Controle de versao"),
    ("git status", "Exibe mudancas no repositorio"),
    ("git add", "Adiciona mudancas ao staging"),
    ("git add --all", "Adiciona todas as alteracoes"),
    ("git commit", "Cria um commit"),
    ("git commit --amend", "Edita o ultimo commit"),
    ("git commit --all", "Inclui arquivos rastreados"),
    ("git checkout", "Troca branch ou restaura arquivos"),
    ("git switch", "Troca de branch"),
    ("git switch -c", "Cria e acessa uma branch"),
    ("git push", "Envia commits ao remoto"),
    ("git pull --ff-only", "Atualiza sem merge commit"),
    ("git fetch --all", "Busca remotos"),
    ("git rebase", "Reaplica commits sobre outra base"),
    ("git log --oneline", "Mostra historico compacto"),
    ("git diff --stat", "Resume arquivos modificados"),
    ("git stash", "Guarda alteracoes temporarias"),
    ("gh", "GitHub CLI"),
    ("gh pr list", "Lista pull requests"),
    ("gh pr create", "Cria um pull request"),
    ("gh pr view --web", "Abre um pull request no navegador"),
    ("gh workflow run", "Dispara um workflow"),
    ("docker", "Gerencia containers"),
    ("docker ps", "Lista containers"),
    (
        "docker compose up -d",
        "Inicializa servicos em segundo plano",
    ),
    ("docker compose down", "Encerra os servicos"),
    ("docker logs -f", "Acompanha os logs"),
    ("cargo", "Gerenciador do Rust"),
    ("cargo build --release", "Compila com otimizacoes"),
    ("cargo check", "Verifica o codigo"),
    ("cargo test", "Executa testes"),
    ("cargo add", "Adiciona dependencia"),
    ("npm", "Gerenciador JavaScript"),
    ("npm install", "Instala dependencias"),
    ("npm run build", "Executa o script build"),
    ("pnpm", "Gerenciador JavaScript"),
    ("pnpm install", "Instala dependencias"),
    ("pnpm dev", "Inicia servidor de desenvolvimento"),
    ("kubectl", "Cliente Kubernetes"),
    ("kubectl get pods", "Lista pods"),
    ("kubectl logs -f", "Acompanha logs"),
    ("rg", "Busca texto com ripgrep"),
    ("rg --files", "Lista arquivos pesquisaveis"),
    ("fd", "Encontra arquivos"),
    ("grep", "Busca padroes em texto"),
    ("find", "Busca arquivos e diretorios"),
    ("ssh", "Acessa servidores"),
];

#[derive(Serialize, Deserialize)]
struct PathCache {
    signature: u64,
    commands: Vec<String>,
}

fn cache_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("EITA_CACHE_DIR") {
        return Some(PathBuf::from(path).join("path-index.json"));
    }
    if let Some(path) = env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(path).join("eita/path-index.json"));
    }
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache/eita/path-index.json"))
}

fn path_signature(directories: &[PathBuf]) -> u64 {
    let mut hasher = DefaultHasher::new();
    env::var_os("PATH").hash(&mut hasher);
    env::var_os("FPATH").hash(&mut hasher);
    for directory in directories {
        directory.hash(&mut hasher);
        if let Ok(meta) = fs::metadata(directory) {
            meta.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|time| time.as_nanos()).hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn discover_commands() -> Vec<String> {
    let directories: Vec<PathBuf> = env::var_os("PATH")
        .map(|paths| env::split_paths(&paths).collect()).unwrap_or_default();
    let signature = path_signature(&directories);
    let cache = cache_path();
    if let Some(ref path) = cache {
        if let Ok(contents) = fs::read(path) {
            if let Ok(index) = serde_json::from_slice::<PathCache>(&contents) {
                if index.signature == signature {
                    return index.commands;
                }
            }
        }
    }
    let mut commands = HashSet::new();
    for directory in &directories {
        let Ok(entries) = fs::read_dir(directory) else { continue };
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else { continue };
            if name.starts_with('.') || name.contains(char::is_whitespace) { continue; }
            let Ok(metadata) = fs::metadata(entry.path()) else { continue };
            if metadata.is_file() && is_executable(&metadata) { commands.insert(name); }
        }
    }
    let mut commands: Vec<String> = commands.into_iter().collect();
    commands.sort_unstable();
    if let Some(path) = cache {
        if let Some(parent) = path.parent() {
            if fs::create_dir_all(parent).is_ok() {
                if let Ok(bytes) = serde_json::to_vec(&PathCache { signature, commands: commands.clone() }) {
                    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
                    if fs::write(&temp, bytes).is_ok() {
                        let _ = fs::rename(&temp, &path);
                    }
                }
            }
        }
    }
    commands
}

pub fn load_catalog() -> Vec<Candidate> {
    let mut catalog = Vec::new();
    let mut seen = HashSet::new();
    for (value, description) in BUILTINS {
        insert(&mut catalog, &mut seen, value, description, "builtin");
    }

    // Cache executable names; refresh only when PATH, FPATH or directory metadata changes.
    for name in discover_commands() {
        insert(&mut catalog, &mut seen, &name, "Executable discovered in PATH", "path");
    }

    if let Some(spec_path) = spec_file() {
        if let Ok(contents) = fs::read_to_string(spec_path) {
            for line in contents.lines() {
                if line.trim().is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((value, description)) = line.split_once('\t') {
                    insert(
                        &mut catalog,
                        &mut seen,
                        value.trim(),
                        description.trim(),
                        "user",
                    );
                }
            }
        }
    }
    catalog
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_: &fs::Metadata) -> bool {
    true
}

fn spec_file() -> Option<PathBuf> {
    if let Some(path) = env::var_os("EITA_SPECS") {
        return Some(path.into());
    }
    if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("eita/specs.tsv"));
    }
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/eita/specs.tsv"))
}

fn insert(
    catalog: &mut Vec<Candidate>,
    seen: &mut HashSet<String>,
    value: &str,
    description: &str,
    source: &str,
) {
    if !value.is_empty() && seen.insert(value.to_owned()) {
        catalog.push(Candidate::new(value, description, source));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_the_command_name() {
        let item = Candidate::new("git commit --amend", "Edita commit", "test");
        assert_eq!(item.root(), "git");
    }
}
