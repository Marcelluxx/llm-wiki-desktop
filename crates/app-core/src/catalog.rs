use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::contracts::{CONTRACT_VERSION, JobState};

const DATABASE_RELATIVE_PATH: &str = ".llm-wiki/catalog.sqlite3";

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("Cannot access the wiki job catalog: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Cannot create a timestamp: {0}")]
    Time(#[from] time::error::Format),
    #[error("Cannot prepare the wiki catalog directory: {0}")]
    Io(#[from] std::io::Error),
}

impl CatalogError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Database(_) | Self::Io(_) => "catalog_unavailable",
            Self::Time(_) => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobSummary {
    pub schema_version: String,
    pub job_id: String,
    pub wiki_id: String,
    pub state: JobState,
    pub stage_progress: f64,
    pub source_count: u32,
    pub created_at: String,
    pub updated_at: String,
    pub last_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WikiCatalog {
    database_path: PathBuf,
}

impl WikiCatalog {
    pub fn open(wiki_root: impl AsRef<Path>) -> Result<Self, CatalogError> {
        let database_path = wiki_root.as_ref().join(DATABASE_RELATIVE_PATH);
        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let catalog = Self { database_path };
        catalog.migrate()?;
        Ok(catalog)
    }

    pub fn create_job(&self, wiki_id: &str, source_count: u32) -> Result<JobSummary, CatalogError> {
        let now = timestamp()?;
        let job = JobSummary {
            schema_version: CONTRACT_VERSION.to_owned(),
            job_id: Uuid::new_v4().to_string(),
            wiki_id: wiki_id.to_owned(),
            state: JobState::Queued,
            stage_progress: 0.0,
            source_count,
            created_at: now.clone(),
            updated_at: now,
            last_message: Some("stage.queued".to_owned()),
        };
        self.connection()?.execute(
            "INSERT INTO jobs (job_id, wiki_id, state, stage_progress, source_count, created_at, updated_at, last_message) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                job.job_id,
                job.wiki_id,
                state_name(job.state),
                job.stage_progress,
                job.source_count,
                job.created_at,
                job.updated_at,
                job.last_message,
            ],
        )?;
        Ok(job)
    }

    pub fn update_job(
        &self,
        job_id: &str,
        state: JobState,
        progress: f64,
        message: Option<&str>,
    ) -> Result<(), CatalogError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "UPDATE jobs SET state = ?2, stage_progress = ?3, updated_at = ?4, last_message = ?5 WHERE job_id = ?1",
            params![job_id, state_name(state), progress.clamp(0.0, 1.0), timestamp()?, message],
        )?;
        transaction.execute(
            "INSERT INTO stage_checkpoints (job_id, state, completed_at)
             SELECT ?1, ?2, ?3 WHERE NOT EXISTS (
               SELECT 1 FROM stage_checkpoints WHERE job_id = ?1 AND state = ?2
             )",
            params![job_id, state_name(state), timestamp()?],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_jobs(&self, wiki_id: &str) -> Result<Vec<JobSummary>, CatalogError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT job_id, wiki_id, state, stage_progress, source_count, created_at, updated_at, last_message FROM jobs WHERE wiki_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([wiki_id], |row| {
            let state: String = row.get(2)?;
            Ok(JobSummary {
                schema_version: CONTRACT_VERSION.to_owned(),
                job_id: row.get(0)?,
                wiki_id: row.get(1)?,
                state: parse_state(&state),
                stage_progress: row.get(3)?,
                source_count: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                last_message: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(CatalogError::from)
    }

    fn migrate(&self) -> Result<(), CatalogError> {
        let connection = self.connection()?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS jobs (
               job_id TEXT PRIMARY KEY,
               wiki_id TEXT NOT NULL,
               state TEXT NOT NULL,
               stage_progress REAL NOT NULL,
               source_count INTEGER NOT NULL,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               last_message TEXT
             );
             CREATE TABLE IF NOT EXISTS source_records (
               source_id TEXT PRIMARY KEY,
               job_id TEXT NOT NULL REFERENCES jobs(job_id) ON DELETE CASCADE,
               original_name TEXT NOT NULL,
               source_format TEXT NOT NULL,
               content_sha256 TEXT,
               byte_size INTEGER,
               relative_path TEXT,
               path_base TEXT NOT NULL DEFAULT 'legacy_wiki_root'
             );
             CREATE TABLE IF NOT EXISTS stage_checkpoints (
               checkpoint_id INTEGER PRIMARY KEY AUTOINCREMENT,
               job_id TEXT NOT NULL REFERENCES jobs(job_id) ON DELETE CASCADE,
               state TEXT NOT NULL,
               completed_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS review_items (
               review_id TEXT PRIMARY KEY,
               job_id TEXT NOT NULL REFERENCES jobs(job_id) ON DELETE CASCADE,
               severity TEXT NOT NULL,
               code TEXT NOT NULL,
               message TEXT NOT NULL,
               status TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS operation_history (
               operation_id INTEGER PRIMARY KEY AUTOINCREMENT,
               job_id TEXT NOT NULL REFERENCES jobs(job_id) ON DELETE CASCADE,
               operation TEXT NOT NULL,
               occurred_at TEXT NOT NULL
             );",
        )?;
        ensure_column(&connection, "source_records", "relative_path", "TEXT")?;
        ensure_column(
            &connection,
            "source_records",
            "path_base",
            "TEXT NOT NULL DEFAULT 'legacy_wiki_root'",
        )?;
        Ok(())
    }

    pub fn recover_interrupted_jobs(&self) -> Result<(), CatalogError> {
        self.connection()?.execute(
            "UPDATE jobs SET state = 'needs_review', last_message = 'stage.interrupted', updated_at = ?1 WHERE state IN ('acquiring', 'extracting', 'ingesting', 'validating', 'staging', 'publishing')",
            [timestamp()?],
        )?;
        Ok(())
    }

    fn connection(&self) -> Result<Connection, CatalogError> {
        Ok(Connection::open(&self.database_path)?)
    }
}

fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), rusqlite::Error> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    connection.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )?;
    Ok(())
}

fn timestamp() -> Result<String, time::error::Format> {
    OffsetDateTime::now_utc().format(&Rfc3339)
}

fn state_name(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Acquiring => "acquiring",
        JobState::Extracting => "extracting",
        JobState::Ingesting => "ingesting",
        JobState::Validating => "validating",
        JobState::Staging => "staging",
        JobState::Publishing => "publishing",
        JobState::Completed => "completed",
        JobState::NeedsReview => "needs_review",
        JobState::Cancelled => "cancelled",
        JobState::Failed => "failed",
    }
}

fn parse_state(value: &str) -> JobState {
    match value {
        "queued" => JobState::Queued,
        "acquiring" => JobState::Acquiring,
        "extracting" => JobState::Extracting,
        "ingesting" => JobState::Ingesting,
        "validating" => JobState::Validating,
        "staging" => JobState::Staging,
        "publishing" => JobState::Publishing,
        "completed" => JobState::Completed,
        "cancelled" => JobState::Cancelled,
        "failed" => JobState::Failed,
        _ => JobState::NeedsReview,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn creates_and_updates_a_durable_job() {
        let directory = tempdir().expect("temporary wiki");
        let catalog = WikiCatalog::open(directory.path()).expect("catalog");
        let job = catalog.create_job("wiki-1", 3).expect("job");
        catalog
            .update_job(
                &job.job_id,
                JobState::Extracting,
                0.5,
                Some("stage.extracting"),
            )
            .expect("progress");

        let jobs = catalog.list_jobs("wiki-1").expect("jobs");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].state, JobState::Extracting);
        assert_eq!(jobs[0].source_count, 3);
    }

    #[test]
    fn marks_interrupted_work_for_safe_review() {
        let directory = tempdir().expect("temporary wiki");
        let catalog = WikiCatalog::open(directory.path()).expect("catalog");
        let job = catalog.create_job("wiki-1", 1).expect("job");
        catalog
            .update_job(
                &job.job_id,
                JobState::Ingesting,
                0.4,
                Some("stage.ingesting"),
            )
            .expect("progress");

        catalog.recover_interrupted_jobs().expect("recovery");

        let jobs = catalog.list_jobs("wiki-1").expect("jobs");
        assert_eq!(jobs[0].state, JobState::NeedsReview);
        assert_eq!(jobs[0].last_message.as_deref(), Some("stage.interrupted"));
    }
}
