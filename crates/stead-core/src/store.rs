//! Append-only journal — the system of record. Indexes and exports
//! are derivations that can always be rebuilt from the journal.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Entity, Observation};
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum JournalEvent {
    UpsertEntity(Entity),
    RetireEntity { id: String, at: String },
    Observe(Observation),
}

/// One journal file per write session, append-only at the file level;
/// existing files are never modified (mazzap convention).
pub struct Journal {
    path: PathBuf,
    file: File,
}

impl Journal {
    /// Open a new journal session file under `dir`.
    pub fn create(dir: &Path, session: &str) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let seq = std::fs::read_dir(dir)?.count() + 1;
        let path = dir.join(format!("{seq:06}-{session}.jsonl"));
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)?;
        Ok(Self { path, file })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&mut self, event: &JournalEvent) -> Result<()> {
        let line = serde_json::to_string(event)?;
        writeln!(self.file, "{line}")?;
        Ok(())
    }

    /// Replay every event across all session files in `dir`, in order.
    pub fn replay(dir: &Path) -> Result<Vec<JournalEvent>> {
        let mut files: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
            .collect();
        files.sort();
        let mut events = Vec::new();
        for path in files {
            for line in BufReader::new(File::open(path)?).lines() {
                let line = line?;
                if !line.trim().is_empty() {
                    events.push(serde_json::from_str(&line)?);
                }
            }
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Observation, Provenance};

    #[test]
    fn journal_roundtrip() {
        let dir = std::env::temp_dir().join(format!("stead-test-{}", uuid::Uuid::new_v4()));
        let mut j = Journal::create(&dir, "test").unwrap();
        j.append(&JournalEvent::Observe(Observation {
            entity_id: "zone:kitchen".into(),
            attr: "temperature_f".into(),
            value: serde_json::json!(77.2),
            provenance: Provenance {
                source: "sensor.kitchen_ambient_temperature".into(),
                run_id: None,
                confidence: None,
                observed_at: "2026-07-10T21:00:00Z".into(),
            },
        }))
        .unwrap();
        let events = Journal::replay(&dir).unwrap();
        assert_eq!(events.len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
