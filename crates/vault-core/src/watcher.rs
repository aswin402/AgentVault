use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<Result<Event, notify::Error>>,
    target_files: HashSet<PathBuf>,
}

impl ConfigWatcher {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel(100);
        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            Config::default(),
        )?;
        Ok(Self {
            watcher,
            rx,
            target_files: HashSet::new(),
        })
    }

    pub fn watch(&mut self, path: PathBuf) -> Result<()> {
        if path.exists() {
            if let Some(parent) = path.parent() {
                self.watcher.watch(parent, RecursiveMode::NonRecursive)?;
                self.target_files.insert(path);
            }
        }
        Ok(())
    }

    pub async fn next_event(&mut self) -> Option<Result<Event, notify::Error>> {
        loop {
            match self.rx.recv().await {
                Some(Ok(event)) => {
                    let matches_target = event.paths.iter().any(|p| self.target_files.contains(p));
                    if matches_target {
                        return Some(Ok(event));
                    }
                }
                Some(Err(e)) => return Some(Err(e)),
                None => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_watcher_detects_changes() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.json");
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, "{{}}").unwrap();
        }

        let mut watcher = ConfigWatcher::new().unwrap();
        watcher.watch(file_path.clone()).unwrap();

        // Write changes
        {
            let mut f = File::create(&file_path).unwrap();
            writeln!(f, "{{\"changed\": true}}").unwrap();
        }

        // Receive event
        let mut detected = false;
        for _ in 0..10 {
            if let Ok(Some(Ok(event))) =
                tokio::time::timeout(Duration::from_millis(100), watcher.next_event()).await
            {
                if event.kind.is_modify() {
                    detected = true;
                    break;
                }
            }
        }
        assert!(detected, "Watcher failed to detect file modification event");
    }

    #[tokio::test]
    async fn test_config_watcher_multiple_files() {
        let dir = tempdir().unwrap();
        let file_path1 = dir.path().join("config1.json");
        let file_path2 = dir.path().join("config2.json");
        {
            let mut f1 = File::create(&file_path1).unwrap();
            writeln!(f1, "{{}}").unwrap();
            let mut f2 = File::create(&file_path2).unwrap();
            writeln!(f2, "{{}}").unwrap();
        }

        let mut watcher = ConfigWatcher::new().unwrap();
        watcher.watch(file_path1.clone()).unwrap();
        watcher.watch(file_path2.clone()).unwrap();

        // Write changes to both files
        {
            let mut f1 = File::create(&file_path1).unwrap();
            writeln!(f1, "{{\"changed\": true}}").unwrap();
            let mut f2 = File::create(&file_path2).unwrap();
            writeln!(f2, "{{\"changed\": true}}").unwrap();
        }

        // Receive events
        let mut detected_paths = HashSet::new();
        for _ in 0..10 {
            if let Ok(Some(Ok(event))) =
                tokio::time::timeout(Duration::from_millis(100), watcher.next_event()).await
            {
                for p in event.paths {
                    detected_paths.insert(p);
                }
            }
        }
        assert!(
            detected_paths.contains(&file_path1),
            "Watcher failed to detect config1.json modification"
        );
        assert!(
            detected_paths.contains(&file_path2),
            "Watcher failed to detect config2.json modification"
        );
    }
}
