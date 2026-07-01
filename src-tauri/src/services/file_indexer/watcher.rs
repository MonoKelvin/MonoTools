use anyhow::Result;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::path::Path;
use tokio::sync::mpsc;

pub struct FileWatcher {
    watcher: notify::RecommendedWatcher,
}

impl FileWatcher {
    pub fn new() -> Result<Self> {
        let (tx, _rx) = mpsc::channel(100);

        let watcher = notify::RecommendedWatcher::try_new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let _ = tx.try_send(event);
                }
            },
            notify::Config::default(),
        )?;

        Ok(Self { watcher })
    }

    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;
        Ok(())
    }

    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.watcher.unwatch(path.as_ref())?;
        Ok(())
    }
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create file watcher")
    }
}
