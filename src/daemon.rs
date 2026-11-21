//! Small on-demand Unix domain socket engine. No TCP, no shell evaluation.
//! Only this user's process can access the runtime directory/socket.
use crate::{
    catalog, completion, engine, inline,
    usage::{self, Usage},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::env;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const MAX_REQUEST: u64 = 4096;
const MAX_CACHE: usize = 256;
const IO_TIMEOUT: Duration = Duration::from_millis(120);
const IDLE_TIMEOUT: Duration = Duration::from_secs(900);

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub op: String,
    #[serde(default)]
    pub input: String,
    #[serde(default)]
    pub limit: usize,
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    pub output: String,
}

fn runtime_path() -> PathBuf {
    if let Some(path) = env::var_os("EITA_DAEMON_SOCKET") {
        return PathBuf::from(path);
    }
    let base = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .or_else(|| env::var_os("XDG_CACHE_HOME").map(PathBuf::from))
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from(format!("/tmp/eita-{}", unsafe { libc::geteuid() })));
    base.join("eita/engine.sock")
}

fn prepare_directory(path: &Path) -> Result<(), Box<dyn Error>> {
    let parent = path.parent().ok_or("socket has no parent")?;
    fs::create_dir_all(parent)?;
    let meta = fs::symlink_metadata(parent)?;
    if !meta.is_dir() || meta.file_type().is_symlink() || meta.uid() != unsafe { libc::geteuid() } {
        return Err("unsafe daemon directory ownership or type".into());
    }
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn connect() -> std::io::Result<UnixStream> {
    let stream = UnixStream::connect(runtime_path())?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    Ok(stream)
}

pub fn query(request: &Request) -> Option<Response> {
    if request.input.len() > MAX_REQUEST as usize || request.path.len() > MAX_REQUEST as usize {
        return None;
    }
    let mut stream = connect().ok()?;
    let mut encoded = serde_json::to_vec(request).ok()?;
    encoded.push(b'\n');
    if encoded.len() > MAX_REQUEST as usize {
        return None;
    }
    stream.write_all(&encoded).ok()?;
    let mut line = Vec::new();
    let len = BufReader::new(stream).read_until(b'\n', &mut line).ok()?;
    if len == 0 || len as u64 > MAX_REQUEST || !line.ends_with(b"\n") {
        return None;
    }
    serde_json::from_slice(&line).ok()
}

pub fn enabled() -> bool {
    env::var("EITA_DAEMON_ENABLED").is_ok_and(|value| value == "1")
}

pub fn inline(left: &str) -> Option<String> {
    if !enabled() || env::var_os("EITA_ALIASES_FILE").is_some() {
        return None;
    }
    let request = Request {
        op: "inline".into(),
        input: left.into(),
        limit: 0,
        path: env::var("PATH").unwrap_or_default(),
    };
    query(&request)
        .filter(|response| response.ok)
        .map(|response| response.output)
}

pub fn suggest(input: &str, limit: usize) -> Option<Vec<engine::Suggestion>> {
    if !enabled() || env::var_os("EITA_ALIASES_FILE").is_some() {
        return None;
    }
    let request = Request {
        op: "suggest".into(),
        input: input.into(),
        limit,
        path: env::var("PATH").unwrap_or_default(),
    };
    let response = query(&request).filter(|response| response.ok)?;
    serde_json::from_str(&response.output).ok()
}

pub fn command(action: &str) -> Result<(), Box<dyn Error>> {
    match action {
        "serve" => serve(),
        "socket" => {
            println!("{}", runtime_path().display());
            Ok(())
        }
        "bench" => benchmark(200),
        "start" => {
            if query(&Request {
                op: "status".into(),
                input: String::new(),
                limit: 0,
                path: String::new(),
            })
            .is_some()
            {
                println!("Eita daemon already running");
                return Ok(());
            }
            let bin = env::current_exe()?;
            let mut command = Command::new(bin);
            command
                .args(["daemon", "serve"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            // Disconnect from the SSH terminal/session. setsid is async-signal-safe.
            use std::os::unix::process::CommandExt;
            unsafe {
                command.pre_exec(|| {
                    if libc::setsid() < 0 {
                        Err(std::io::Error::last_os_error())
                    } else {
                        Ok(())
                    }
                });
            }
            let child = command.spawn()?;
            for _ in 0..40 {
                if query(&Request {
                    op: "status".into(),
                    input: String::new(),
                    limit: 0,
                    path: String::new(),
                })
                .is_some()
                {
                    println!("Eita daemon started: pid={}", child.id());
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err("daemon failed to start (check socket directory ownership)".into())
        }
        "stop" | "status" | "reload" => {
            let response = query(&Request {
                op: action.into(),
                input: String::new(),
                limit: 0,
                path: String::new(),
            })
            .ok_or("daemon offline")?;
            if !response.ok {
                return Err(response.output.into());
            }
            println!("{}", response.output);
            Ok(())
        }
        _ => Err("unknown daemon command".into()),
    }
}

struct Engine {
    catalog: Vec<catalog::Candidate>,
    usage: Usage,
    started: Instant,
    last_activity: Instant,
    next_check: Instant,
    completion_stamp: Option<SystemTime>,
    usage_stamp: Option<SystemTime>,
    hits: u64,
    misses: u64,
    queue: VecDeque<String>,
    cache: HashMap<String, String>,
    path: String,
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

impl Engine {
    fn new() -> Self {
        let now = Instant::now();
        let mut engine = Self {
            catalog: catalog::load_catalog(),
            usage: Usage::load_saved(),
            started: now,
            last_activity: now,
            next_check: now + Duration::from_secs(1),
            completion_stamp: modified(&completion::cache_path()),
            usage_stamp: modified(&usage::usage_path()),
            hits: 0,
            misses: 0,
            queue: VecDeque::new(),
            cache: HashMap::new(),
            path: env::var("PATH").unwrap_or_default(),
        };
        engine.merge_learned();
        engine
    }

    fn merge_learned(&mut self) {
        for entry in self.usage.learned() {
            if !self
                .catalog
                .iter()
                .any(|candidate| candidate.value == entry.value)
            {
                self.catalog.push(entry);
            }
        }
    }

    fn reload(&mut self) {
        self.catalog = catalog::load_catalog();
        self.usage = Usage::load_saved();
        self.merge_learned();
        self.completion_stamp = modified(&completion::cache_path());
        self.usage_stamp = modified(&usage::usage_path());
        self.cache.clear();
        self.queue.clear();
    }

    fn refresh_if_stale(&mut self) {
        if Instant::now() < self.next_check {
            return;
        }
        self.next_check = Instant::now() + Duration::from_secs(1);
        if self.completion_stamp != modified(&completion::cache_path())
            || self.usage_stamp != modified(&usage::usage_path())
        {
            self.reload();
        }
    }

    fn cached(&mut self, key: String, calc: impl FnOnce(&Self) -> String) -> String {
        if let Some(value) = self.cache.get(&key).cloned() {
            self.hits += 1;
            if let Some(index) = self.queue.iter().position(|stored| stored == &key) {
                self.queue.remove(index);
            }
            self.queue.push_back(key);
            return value;
        }
        self.misses += 1;
        let value = calc(self);
        if self.queue.len() >= MAX_CACHE {
            if let Some(old) = self.queue.pop_front() {
                self.cache.remove(&old);
            }
        }
        self.queue.push_back(key.clone());
        self.cache.insert(key, value.clone());
        value
    }

    fn respond(&mut self, request: Request) -> Response {
        self.last_activity = Instant::now();
        self.refresh_if_stale();
        if request.input.len() > 1024 || request.limit > 100 || request.path.len() > 4096 {
            return Response {
                ok: false,
                output: "request too large".into(),
            };
        }
        if matches!(request.op.as_str(), "suggest" | "inline") && request.path != self.path {
            // Per-shell PATH may differ; client must run its own local fallback.
            return Response {
                ok: false,
                output: "PATH changed; local fallback required".into(),
            };
        }
        let result = match request.op.as_str() {
            "inline" => self.cached(format!("i:{}", request.input), |engine| {
                inline::suffix(&engine.catalog, &request.input, &engine.usage).unwrap_or_default()
            }),
            "suggest" => self.cached(format!("s:{}:{}", request.limit, request.input), |engine| {
                serde_json::to_string(&engine::suggest_ranked(
                    &engine.catalog,
                    &request.input,
                    request.limit,
                    &engine.usage,
                ))
                .unwrap_or_else(|_| "[]".into())
            }),
            "reload" => {
                self.reload();
                format!("reloaded {} candidates", self.catalog.len())
            }
            "status" => serde_json::json!({
                "pid": std::process::id(),
                "uptime_ms": self.started.elapsed().as_millis(),
                "candidates": self.catalog.len(),
                "cache_entries": self.cache.len(),
                "hits": self.hits,
                "misses": self.misses,
                "idle_timeout_seconds": IDLE_TIMEOUT.as_secs(),
                "socket": runtime_path().display().to_string(),
            })
            .to_string(),
            "stop" => "stopping".into(),
            _ => {
                return Response {
                    ok: false,
                    output: "invalid operation".into(),
                }
            }
        };
        Response {
            ok: true,
            output: result,
        }
    }
}

pub fn serve() -> Result<(), Box<dyn Error>> {
    let socket = runtime_path();
    prepare_directory(&socket)?;
    if let Ok(meta) = fs::symlink_metadata(&socket) {
        if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
            return Err("unsafe existing socket".into());
        }
        if connect().is_ok() {
            return Err("daemon already running".into());
        }
        fs::remove_file(&socket)?;
    }
    let listener = UnixListener::bind(&socket)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
    let mut engine = Engine::new();
    let mut should_stop = false;
    while !should_stop && engine.last_activity.elapsed() < IDLE_TIMEOUT {
        let mut pollfd = libc::pollfd {
            fd: listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // poll wakes immediately on a request; unlike a busy sleep, it uses no CPU while idle.
        let ready = unsafe { libc::poll(&mut pollfd, 1, 1000) };
        if ready <= 0 {
            continue;
        }
        let (mut stream, _) = match listener.accept() {
            Ok(stream) => stream,
            Err(_) => continue,
        };
        stream.set_read_timeout(Some(IO_TIMEOUT))?;
        stream.set_write_timeout(Some(IO_TIMEOUT))?;
        let mut raw = Vec::new();
        let size = BufReader::new(&stream).read_until(b'\n', &mut raw);
        if !matches!(size, Ok(n) if n > 0 && n as u64 <= MAX_REQUEST) || !raw.ends_with(b"\n") {
            continue;
        }
        if raw.starts_with(b"I\t") {
            let contents = String::from_utf8_lossy(&raw);
            let mut items = contents.trim_end_matches('\n').splitn(3, '\t');
            let _kind = items.next();
            let path = items.next().unwrap_or("");
            let left = items.next().unwrap_or("");
            // Plain ZLE protocol: '.' means valid, '!' means fallback.
            let result = if path == engine.path
                && left.len() <= 1024
                && !left.chars().any(|ch| ch.is_control())
            {
                let result = engine.cached(format!("i:{left}"), |state| {
                    inline::suffix(&state.catalog, left, &state.usage).unwrap_or_default()
                });
                engine.last_activity = Instant::now();
                format!(".{result}\n")
            } else {
                "!\n".into()
            };
            let _ = stream.write_all(result.as_bytes());
            continue;
        }
        let Ok(request) = serde_json::from_slice::<Request>(&raw) else {
            continue;
        };
        should_stop = request.op == "stop";
        let response = engine.respond(request);
        let mut output = serde_json::to_vec(&response)?;
        output.push(b'\n');
        let _ = stream.write_all(&output);
    }
    // Only delete our own listener, avoiding races with another server.
    let _ = fs::remove_file(socket);
    Ok(())
}

fn benchmark(iterations: usize) -> Result<(), Box<dyn Error>> {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let before = Instant::now();
        let result = query(&Request {
            op: "suggest".into(),
            input: "git com".into(),
            limit: 5,
            path: env::var("PATH").unwrap_or_default(),
        })
        .ok_or("daemon request timed out")?;
        if !result.ok {
            return Err(result.output.into());
        }
        samples.push(before.elapsed().as_micros());
    }
    samples.sort_unstable();
    println!(
        "{}",
        serde_json::json!({
            "iterations": iterations,
            "p50_us": samples[iterations / 2],
            "p95_us": samples[(iterations - 1) * 95 / 100],
            "p99_us": samples[(iterations - 1) * 99 / 100],
            "scope": "CLI-to-daemon IPC incl encoding/matcher; excludes process startup and ZLE rendering",
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_is_bounded_and_reuses_identical_queries() {
        let mut engine = Engine::new();
        let first = engine.cached("same".into(), |_| "hello".into());
        let second = engine.cached("same".into(), |_| "bad".into());
        assert_eq!(first, second);
        assert_eq!(engine.hits, 1);
        assert_eq!(engine.misses, 1);
        for i in 0..(MAX_CACHE + 1) {
            engine.cached(format!("{i}"), |_| "x".into());
        }
        assert!(engine.cache.len() <= MAX_CACHE);
    }
}
