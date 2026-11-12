//! Die Speicherstände der Runs: je Run eine Datei `runs/<seed>.json` im Konfigurationsordner neben `settings.json`,
//! geschrieben nach jeder Änderung des Runs, gelöscht, wenn er endet oder abgebrochen wird. Eine unlesbare Datei gilt
//! als keine.

use std::{cmp::Reverse, fs, io, path::PathBuf};

use bevy::prelude::*;
use voidfleet_core::run::Run;

use crate::settings::config_file;

/// Der Ordner der Speicherstände; `None` speichert nichts (Rauchtest, Vorschau-Szenen).
#[derive(Resource, Clone, Debug)]
pub struct RunSave(pub Option<PathBuf>);

impl Default for RunSave {
    fn default() -> Self {
        Self(config_file("runs"))
    }
}

impl RunSave {
    fn file(&self, seed: u64) -> Option<PathBuf> {
        Some(self.0.as_ref()?.join(format!("{seed}.json")))
    }

    /// Die gespeicherten, nicht beendeten Runs, der zuletzt gespielte zuerst; unlesbare Dateien zählen nicht.
    pub fn list(&self) -> Vec<Run> {
        let Some(entries) = self.0.as_ref().and_then(|dir| fs::read_dir(dir).ok()) else {
            return Vec::new();
        };
        let mut runs: Vec<_> = entries
            .flatten()
            .filter_map(|entry| {
                let json = fs::read_to_string(entry.path()).ok()?;
                match serde_json::from_str::<Run>(&json) {
                    Ok(run) => run
                        .result
                        .is_none()
                        .then_some((entry.metadata().and_then(|file| file.modified()).ok()?, run)),
                    Err(error) => {
                        warn!(
                            "Speicherstand {:?} unlesbar, wie keiner: {error}",
                            entry.path()
                        );
                        None
                    }
                }
            })
            .collect();
        runs.sort_by_key(|&(played, _)| Reverse(played));
        runs.into_iter().map(|(_, run)| run).collect()
    }

    pub fn store(&self, run: &Run) -> io::Result<()> {
        let Some(path) = self.file(run.seed) else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, serde_json::to_string(run)?)
    }

    /// Löscht den Speicherstand des Runs mit diesem Seed.
    pub fn clear(&self, seed: u64) {
        if let Some(path) = self.file(seed)
            && let Err(error) = fs::remove_file(path)
            && error.kind() != io::ErrorKind::NotFound
        {
            error!("Speicherstand nicht gelöscht: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use voidfleet_core::admirals::AdmiralId;

    use super::*;

    fn temp_save(name: &str) -> RunSave {
        let dir = std::env::temp_dir().join(format!("voidfleet-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        RunSave(Some(dir))
    }

    #[test]
    fn store_list_clear() {
        let save = temp_save("save");
        assert_eq!(save.list(), []);
        let mut run = Run::new(AdmiralId::Vance, 42);
        run.advance(run.choices()[0]).unwrap();
        save.store(&run).unwrap();
        assert_eq!(save.list(), [run.clone()]);
        // Dasselbe Gefecht nach dem Laden.
        assert_eq!(
            save.list()[0].start_battle().enemies,
            run.start_battle().enemies
        );
        // Ein zweiter Run steht daneben; jeder wird für sich gelöscht.
        let other = Run::new(AdmiralId::Kerr, 43);
        save.store(&other).unwrap();
        assert_eq!(save.list().len(), 2);
        save.clear(run.seed);
        assert_eq!(save.list(), [other]);
        save.clear(run.seed);
    }

    #[test]
    fn unreadable_or_finished_is_none() {
        let save = temp_save("broken");
        let dir = save.0.clone().unwrap();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("1.json"), "{ kein json").unwrap();
        let mut run = Run::new(AdmiralId::Osei, 2);
        run.result = Some(voidfleet_core::battle::BattleResult::Lost);
        save.store(&run).unwrap();
        assert_eq!(save.list(), []);
        assert_eq!(RunSave(None).list(), []);
    }
}
