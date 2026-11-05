//! Der Speicherstand des Runs: `run.json` im Konfigurationsordner neben `settings.json`, geschrieben nach jeder
//! Änderung des Runs, gelöscht, wenn er endet oder abgebrochen wird. Eine unlesbare Datei gilt als keine.

use std::{fs, io, path::PathBuf};

use bevy::prelude::*;
use voidfleet_core::run::Run;

use crate::settings::config_file;

/// Wo der Run gespeichert wird; `None` speichert nichts (Rauchtest, Vorschau-Szenen).
#[derive(Resource, Clone, Debug)]
pub struct RunSave(pub Option<PathBuf>);

impl Default for RunSave {
    fn default() -> Self {
        Self(config_file("run.json"))
    }
}

impl RunSave {
    /// Der gespeicherte, nicht beendete Run; ohne Datei, bei unlesbarer Datei oder beendetem Run keiner.
    pub fn load(&self) -> Option<Run> {
        let json = fs::read_to_string(self.0.as_ref()?).ok()?;
        match serde_json::from_str::<Run>(&json) {
            Ok(run) => run.result.is_none().then_some(run),
            Err(error) => {
                warn!("Speicherstand unlesbar, wie keiner: {error}");
                None
            }
        }
    }

    pub fn store(&self, run: &Run) -> io::Result<()> {
        let Some(path) = &self.0 else {
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, serde_json::to_string(run)?)
    }

    pub fn clear(&self) {
        if let Some(path) = &self.0
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
        RunSave(Some(dir.join("run.json")))
    }

    #[test]
    fn store_load_clear() {
        let save = temp_save("save");
        assert_eq!(save.load(), None);
        let mut run = Run::new(AdmiralId::Vance, 42);
        run.advance(run.choices()[0]).unwrap();
        save.store(&run).unwrap();
        assert_eq!(save.load(), Some(run.clone()));
        // Dasselbe Gefecht nach dem Laden.
        assert_eq!(
            save.load().unwrap().start_battle().enemies,
            run.start_battle().enemies
        );
        save.clear();
        assert_eq!(save.load(), None);
        save.clear();
    }

    #[test]
    fn unreadable_or_finished_is_none() {
        let save = temp_save("broken");
        let path = save.0.clone().unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ kein json").unwrap();
        assert_eq!(save.load(), None);
        let mut run = Run::new(AdmiralId::Osei, 1);
        run.result = Some(voidfleet_core::battle::BattleResult::Lost);
        save.store(&run).unwrap();
        assert_eq!(save.load(), None);
        assert_eq!(RunSave(None).load(), None);
    }
}
