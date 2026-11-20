use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
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

pub fn load_catalog() -> Vec<Candidate> {
    load_with_path(true)
}

/// For the ZLE redraw hook, avoid repeated filesystem scans of PATH.
pub fn load_inline_catalog() -> Vec<Candidate> {
    load_with_path(false)
}

fn load_with_path(scan_path: bool) -> Vec<Candidate> {
    let mut catalog = Vec::new();
    let mut seen = HashSet::new();
    for (value, description) in BUILTINS {
        insert(&mut catalog, &mut seen, value, description, "builtin");
    }

    // Discover command names without invoking any executable.
    if scan_path {
        if let Some(paths) = env::var_os("PATH") {
            for directory in env::split_paths(&paths) {
                let Ok(entries) = fs::read_dir(directory) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let Some(filename) = entry.file_name().to_str().map(str::to_owned) else {
                        continue;
                    };
                    if filename.starts_with('.') || filename.contains(char::is_whitespace) {
                        continue;
                    }
                    let Ok(metadata) = fs::metadata(entry.path()) else {
                        continue;
                    };
                    if !metadata.is_file() || !is_executable(&metadata) {
                        continue;
                    }
                    insert(
                        &mut catalog,
                        &mut seen,
                        &filename,
                        "Executavel encontrado no PATH",
                        "path",
                    );
                }
            }
        }
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

    // A ZLE adapter may export a sanitized tab-delimited list of aliases.
    // Treat aliases as data; never evaluate or execute their expansions.
    if let Some(path) = env::var_os("EITA_ALIASES_FILE") {
        if let Ok(contents) = fs::read_to_string(path) {
            for line in contents.lines() {
                if let Some((name, description)) = line.split_once('\t') {
                    insert(
                        &mut catalog,
                        &mut seen,
                        name.trim(),
                        description.trim(),
                        "alias",
                    );
                }
            }
        }
    }

    // Cached static Zsh definitions are data, never executed as shell code.
    for completion in crate::completion::load() {
        insert(
            &mut catalog,
            &mut seen,
            &completion.value,
            &completion.description,
            &completion.source,
        );
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
