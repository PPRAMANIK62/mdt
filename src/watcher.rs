//! File-system watcher with debounced event delivery.
//!
//! A background thread owns a [`notify::RecommendedWatcher`], coalesces raw
//! events over a 300 ms window (to absorb vim's write-tmp-rename pattern), and
//! sends [`FsEvent`] values to the main thread via `std::sync::mpsc`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// High-level filesystem event delivered to the application.
#[derive(Debug)]
pub enum FsEvent {
    FileModified(PathBuf),
    EntryCreated { path: PathBuf, is_dir: bool },
    EntryRemoved(PathBuf),
    EntryRenamed { from: PathBuf, to: PathBuf },
}

/// Handle returned by [`spawn_watcher`]; call [`shutdown()`](WatcherHandle::shutdown)
/// to stop the background thread.
pub struct WatcherHandle {
    shutdown_tx: mpsc::Sender<()>,
}

impl WatcherHandle {
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Internal kind used while coalescing raw notify events.
#[derive(Debug, Clone)]
enum PendingKind {
    Modified,
    Created { is_dir: bool },
    Removed,
    RenamedFrom,
    RenamedTo,
    RenamedBoth { from: PathBuf },
}

const DEBOUNCE_MS: u64 = 300;
const POLL_MS: u64 = 50;

/// Spawn the watcher thread and return an event receiver + shutdown handle.
pub fn spawn_watcher(root: &Path) -> anyhow::Result<(mpsc::Receiver<FsEvent>, WatcherHandle)> {
    let (fs_tx, fs_rx) = mpsc::channel::<FsEvent>();
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    // Raw channel from notify → debounce thread.
    let (raw_tx, raw_rx) = mpsc::channel::<notify::Event>();

    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = raw_tx.send(event);
            }
        })?;

    watcher.watch(root, RecursiveMode::Recursive)?;

    std::thread::Builder::new().name("fs-watcher".into()).spawn(move || {
        // Keep watcher alive for the lifetime of this thread.
        let _watcher = watcher;
        let mut pending: HashMap<PathBuf, (PendingKind, Instant)> = HashMap::new();

        loop {
            // Check for shutdown.
            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            // Drain all available raw events.
            while let Ok(event) = raw_rx.try_recv() {
                process_raw_event(&event, &mut pending);
            }

            // Flush entries older than debounce window.
            let now = Instant::now();
            let debounce = Duration::from_millis(DEBOUNCE_MS);
            let ready: Vec<PathBuf> = pending
                .iter()
                .filter(|(_, (_, ts))| now.duration_since(*ts) >= debounce)
                .map(|(p, _)| p.clone())
                .collect();

            for path in ready {
                if let Some((kind, _)) = pending.remove(&path) {
                    let _ = fs_tx.send(to_fs_event(path, kind));
                }
            }

            std::thread::sleep(Duration::from_millis(POLL_MS));
        }
    })?;

    Ok((fs_rx, WatcherHandle { shutdown_tx }))
}

/// Returns `true` if the path should be ignored (dotfiles, swap files, etc.).
fn should_ignore(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    name.starts_with('.')
        || name.ends_with('~')
        || Path::new(name).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("swp"))
        || Path::new(name).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("swx"))
}

fn process_raw_event(
    event: &notify::Event,
    pending: &mut HashMap<PathBuf, (PendingKind, Instant)>,
) {
    let now = Instant::now();

    for path in &event.paths {
        if should_ignore(path) {
            continue;
        }

        match &event.kind {
            EventKind::Create(create_kind) => {
                let is_dir = matches!(create_kind, notify::event::CreateKind::Folder);
                pending.insert(path.clone(), (PendingKind::Created { is_dir }, now));
            }
            EventKind::Modify(modify_kind) => {
                match modify_kind {
                    notify::event::ModifyKind::Name(rename_mode) => {
                        match rename_mode {
                            notify::event::RenameMode::Both => {
                                // inotify provides both paths in event.paths[0] = from, [1] = to.
                                // We handle this at the event level below, after the per-path loop.
                            }
                            notify::event::RenameMode::From => {
                                pending.insert(path.clone(), (PendingKind::RenamedFrom, now));
                            }
                            notify::event::RenameMode::To => {
                                pending.insert(path.clone(), (PendingKind::RenamedTo, now));
                            }
                            _ => {
                                // Treat any other rename as modification.
                                pending.insert(path.clone(), (PendingKind::Modified, now));
                            }
                        }
                    }
                    _ => {
                        // Data or metadata modification.
                        // Don't overwrite a Create — the Create already implies new content.
                        pending
                            .entry(path.clone())
                            .and_modify(|(kind, ts)| {
                                if !matches!(kind, PendingKind::Created { .. }) {
                                    *kind = PendingKind::Modified;
                                }
                                *ts = now;
                            })
                            .or_insert((PendingKind::Modified, now));
                    }
                }
            }
            EventKind::Remove(_) => {
                pending.insert(path.clone(), (PendingKind::Removed, now));
            }
            _ => {}
        }
    }

    // Handle RenameMode::Both — event.paths has [from, to].
    if let EventKind::Modify(notify::event::ModifyKind::Name(notify::event::RenameMode::Both)) =
        &event.kind
    {
        if event.paths.len() == 2 {
            let from = &event.paths[0];
            let to = &event.paths[1];
            if !should_ignore(from) && !should_ignore(to) {
                // Remove any per-path entries we may have inserted above (the loop
                // above doesn't insert for RenameMode::Both, but be defensive).
                pending.remove(from);
                pending.insert(to.clone(), (PendingKind::RenamedBoth { from: from.clone() }, now));
            }
        }
    }
}

fn to_fs_event(path: PathBuf, kind: PendingKind) -> FsEvent {
    match kind {
        PendingKind::Modified => FsEvent::FileModified(path),
        PendingKind::Created { is_dir } => FsEvent::EntryCreated { path, is_dir },
        PendingKind::Removed => FsEvent::EntryRemoved(path),
        PendingKind::RenamedFrom => FsEvent::EntryRemoved(path),
        PendingKind::RenamedTo => {
            let is_dir = path.is_dir();
            FsEvent::EntryCreated { path, is_dir }
        }
        PendingKind::RenamedBoth { from } => FsEvent::EntryRenamed { from, to: path },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
    use notify::Event;
    use std::path::PathBuf;

    // ── should_ignore ────────────────────────────────────────────

    #[test]
    fn ignore_dotfile() {
        assert!(should_ignore(Path::new("/tmp/.hidden")));
    }

    #[test]
    fn ignore_backup_tilde() {
        assert!(should_ignore(Path::new("/tmp/file.md~")));
    }

    #[test]
    fn ignore_swp_case_insensitive() {
        assert!(should_ignore(Path::new("/tmp/file.SWP")));
        assert!(should_ignore(Path::new("/tmp/file.swx")));
    }

    #[test]
    fn not_ignore_regular_md() {
        assert!(!should_ignore(Path::new("/tmp/readme.md")));
    }

    #[test]
    fn ignore_no_filename() {
        assert!(should_ignore(Path::new("")));
    }

    // ── process_raw_event ────────────────────────────────────────

    #[test]
    fn process_create_file() {
        let mut pending = HashMap::new();
        let mut event = Event::new(EventKind::Create(CreateKind::File));
        event.paths.push(PathBuf::from("/tmp/test.md"));
        process_raw_event(&event, &mut pending);

        let (kind, _) = pending.get(Path::new("/tmp/test.md")).unwrap();
        assert!(matches!(kind, PendingKind::Created { is_dir: false }));
    }

    #[test]
    fn process_create_folder() {
        let mut pending = HashMap::new();
        let mut event = Event::new(EventKind::Create(CreateKind::Folder));
        event.paths.push(PathBuf::from("/tmp/subdir"));
        process_raw_event(&event, &mut pending);

        let (kind, _) = pending.get(Path::new("/tmp/subdir")).unwrap();
        assert!(matches!(kind, PendingKind::Created { is_dir: true }));
    }

    #[test]
    fn process_modify_no_overwrite_create() {
        let mut pending = HashMap::new();

        // First: create event.
        let mut create_ev = Event::new(EventKind::Create(CreateKind::File));
        create_ev.paths.push(PathBuf::from("/tmp/test.md"));
        process_raw_event(&create_ev, &mut pending);

        let (_, ts_before) = pending.get(Path::new("/tmp/test.md")).unwrap();
        let ts_before = *ts_before;

        // Second: modify event — should NOT overwrite Created kind.
        let mut modify_ev =
            Event::new(EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Content)));
        modify_ev.paths.push(PathBuf::from("/tmp/test.md"));
        process_raw_event(&modify_ev, &mut pending);

        let (kind, ts_after) = pending.get(Path::new("/tmp/test.md")).unwrap();
        assert!(matches!(kind, PendingKind::Created { .. }));
        assert!(*ts_after >= ts_before);
    }

    #[test]
    fn process_remove() {
        let mut pending = HashMap::new();
        let mut event = Event::new(EventKind::Remove(RemoveKind::Any));
        event.paths.push(PathBuf::from("/tmp/test.md"));
        process_raw_event(&event, &mut pending);

        let (kind, _) = pending.get(Path::new("/tmp/test.md")).unwrap();
        assert!(matches!(kind, PendingKind::Removed));
    }

    #[test]
    fn process_rename_both() {
        let mut pending = HashMap::new();
        let mut event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)));
        event.paths.push(PathBuf::from("/tmp/old.md"));
        event.paths.push(PathBuf::from("/tmp/new.md"));
        process_raw_event(&event, &mut pending);

        // `from` path should be absent.
        assert!(!pending.contains_key(Path::new("/tmp/old.md")));
        // `to` path should have RenamedBoth with the correct `from`.
        let (kind, _) = pending.get(Path::new("/tmp/new.md")).unwrap();
        match kind {
            PendingKind::RenamedBoth { from } => {
                assert_eq!(from, Path::new("/tmp/old.md"));
            }
            other => panic!("expected RenamedBoth, got {other:?}"),
        }
    }

    #[test]
    fn process_ignores_dotfile() {
        let mut pending = HashMap::new();
        let mut event = Event::new(EventKind::Create(CreateKind::File));
        event.paths.push(PathBuf::from("/tmp/.gitignore"));
        process_raw_event(&event, &mut pending);

        assert!(pending.is_empty());
    }

    // ── to_fs_event ──────────────────────────────────────────────

    #[test]
    fn to_fs_event_modified() {
        let ev = to_fs_event(PathBuf::from("/tmp/test.md"), PendingKind::Modified);
        assert!(matches!(ev, FsEvent::FileModified(_)));
    }

    #[test]
    fn to_fs_event_renamed_from_becomes_removed() {
        let ev = to_fs_event(PathBuf::from("/tmp/test.md"), PendingKind::RenamedFrom);
        assert!(matches!(ev, FsEvent::EntryRemoved(_)));
    }
}
