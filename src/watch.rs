//! Event-driven indexing of PATH and fpath directories.
//! A bounded background worker handles filesystem notifications, never the ZLE hot path.
use crate::completion;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

const DEBOUNCE: Duration = Duration::from_millis(350);
const MAX_DIRECTORIES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sources {
    pub path: String,
    pub fpath: Option<String>,
    pub manpath: Option<String>,
}

impl Sources {
    pub fn from_env() -> Self {
        let (cached_fpath, cached_manpath) = completion::indexed_sources();
        Self {
            path: env::var("PATH").unwrap_or_default(),
            fpath: env::var("EITA_FPATH")
                .ok()
                .or_else(|| env::var("FPATH").ok())
                .or(cached_fpath),
            manpath: env::var("EITA_MANPATH").ok().or(cached_manpath),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Changed {
    Path,
    Index,
}

pub struct Monitor {
    pub updates: Sender<Sources>,
    pub changes: Receiver<Changed>,
}

pub fn start(sources: Sources) -> Monitor {
    let (update_tx, update_rx) = mpsc::channel();
    let (change_tx, change_rx) = mpsc::channel();
    thread::spawn(move || run(sources, update_rx, change_tx));
    Monitor {
        updates: update_tx,
        changes: change_rx,
    }
}

fn paths(value: &str) -> impl Iterator<Item = PathBuf> + '_ {
    value
        .split(':')
        .filter(|entry| !entry.is_empty())
        .take(128)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.is_dir())
}

fn directories(sources: &Sources) -> Vec<(PathBuf, u8)> {
    let mut folders = HashMap::<PathBuf, u8>::new();
    for path in paths(&sources.path) {
        *folders.entry(path).or_default() |= 1;
    }
    if let Some(fpath) = &sources.fpath {
        for path in paths(fpath) {
            *folders.entry(path).or_default() |= 2;
        }
    }
    if let Some(manpath) = &sources.manpath {
        for path in paths(manpath) {
            for folder in [path.clone(), path.join("man1"), path.join("man8")] {
                if folder.is_dir() {
                    *folders.entry(folder).or_default() |= 2;
                }
            }
        }
    }
    let mut result: Vec<_> = folders.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result.truncate(MAX_DIRECTORIES);
    result
}

fn refresh(sources: &Sources, index_dirty: bool, path_dirty: bool, tx: &Sender<Changed>) {
    if index_dirty {
        if let Ok(report) = completion::rebuild(
            sources.fpath.as_deref(),
            sources.manpath.as_deref(),
            &completion::cache_path(),
        ) {
            if !report.unchanged {
                let _ = tx.send(Changed::Index);
            }
        }
    }
    if path_dirty {
        let _ = tx.send(Changed::Path);
    }
}

#[cfg(target_os = "linux")]
fn run(mut sources: Sources, rx: Receiver<Sources>, tx: Sender<Changed>) {
    use std::ffi::CString;
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;

    let raw = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
    if raw < 0 {
        // No polling fallback: filesystem monitoring is genuinely event-driven.
        while let Ok(next) = rx.recv() {
            sources = next;
            refresh(&sources, true, true, &tx);
        }
        return;
    }
    let fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(raw) };
    let mut handles = HashMap::<i32, u8>::new();
    let mut directories_known = HashSet::new();
    let mut rebuild_watches = true;
    let mut pending_index = sources.fpath.is_some() || sources.manpath.is_some();
    let mut pending_path = false;
    let mut due = Some(Instant::now() + DEBOUNCE);
    loop {
        loop {
            let next = match rx.try_recv() {
                Ok(next) => next,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            };
            if next != sources {
                pending_index |= next.fpath != sources.fpath || next.manpath != sources.manpath;
                pending_path |= next.path != sources.path;
                sources = next;
                rebuild_watches = true;
                due = Some(Instant::now() + DEBOUNCE);
            }
        }
        if rebuild_watches {
            for watch in handles.keys() {
                unsafe {
                    libc::inotify_rm_watch(std::os::fd::AsRawFd::as_raw_fd(&fd), *watch);
                }
            }
            handles.clear();
            directories_known.clear();
            for (path, kind) in directories(&sources) {
                if !directories_known.insert(path.clone()) {
                    continue;
                }
                let Ok(name) = CString::new(path.as_os_str().as_bytes()) else {
                    continue;
                };
                let mask = libc::IN_CREATE
                    | libc::IN_DELETE
                    | libc::IN_MOVED_TO
                    | libc::IN_MOVED_FROM
                    | libc::IN_CLOSE_WRITE
                    | libc::IN_ATTRIB
                    | libc::IN_DELETE_SELF
                    | libc::IN_MOVE_SELF;
                let wd = unsafe {
                    libc::inotify_add_watch(
                        std::os::fd::AsRawFd::as_raw_fd(&fd),
                        name.as_ptr(),
                        mask,
                    )
                };
                if wd >= 0 {
                    handles.insert(wd, kind);
                }
            }
            rebuild_watches = false;
        }
        let mut pollfd = libc::pollfd {
            fd: std::os::fd::AsRawFd::as_raw_fd(&fd),
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut pollfd, 1, 100) };
        if ready > 0 && pollfd.revents & libc::POLLIN != 0 {
            let mut buffer = [0u8; 8192];
            let count = unsafe {
                libc::read(
                    std::os::fd::AsRawFd::as_raw_fd(&fd),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                )
            };
            if count > 0 {
                let mut offset = 0usize;
                while offset + std::mem::size_of::<libc::inotify_event>() <= count as usize {
                    let event = unsafe {
                        std::ptr::read_unaligned(
                            buffer.as_ptr().add(offset).cast::<libc::inotify_event>(),
                        )
                    };
                    let kind = handles.get(&event.wd).copied().unwrap_or(0);
                    pending_path |= kind & 1 != 0;
                    pending_index |= kind & 2 != 0;
                    offset += std::mem::size_of::<libc::inotify_event>() + event.len as usize;
                }
                due = Some(Instant::now() + DEBOUNCE);
            }
        }
        if due.is_some_and(|moment| Instant::now() >= moment) {
            if pending_index || pending_path {
                refresh(&sources, pending_index, pending_path, &tx);
            }
            pending_index = false;
            pending_path = false;
            due = None;
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn run(mut sources: Sources, rx: Receiver<Sources>, tx: Sender<Changed>) {
    // Other platforms remain manual unless explicitly synchronized by Zsh.
    while let Ok(next) = rx.recv() {
        let changed = next != sources;
        sources = next;
        if changed {
            refresh(&sources, true, true, &tx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_directory_roles_without_recursive_scans() {
        let sources = Sources {
            path: "/usr/bin:/bin".into(),
            fpath: Some("/usr/bin".into()),
            manpath: None,
        };
        let all = directories(&sources);
        assert!(all.len() <= 2);
        for (path, kind) in all {
            if path == std::path::Path::new("/usr/bin") {
                assert_eq!(kind & 3, 3);
            }
        }
    }

    #[test]
    fn refuses_relative_directories() {
        assert!(paths(".:/tmp:relative").all(|path| path.is_absolute()));
    }
}
