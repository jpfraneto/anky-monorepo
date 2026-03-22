use crate::db::queries;
use crate::state::AppState;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use std::fs;
use std::path::Path;

const WRITINGS_DIR: &str = "data/writings";

pub fn save_writing_to_file(wallet_address: &str, timestamp: i64, content: &str) -> Result<()> {
    let wallet_dir = Path::new(WRITINGS_DIR).join(wallet_address.trim());
    fs::create_dir_all(&wallet_dir)
        .with_context(|| format!("failed to create {}", wallet_dir.display()))?;

    let file_path = wallet_dir.join(format!("{}.txt", timestamp));
    fs::write(&file_path, content)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    Ok(())
}

pub async fn backfill_writings_to_files(state: &AppState) -> Result<()> {
    let archive_dir = Path::new(WRITINGS_DIR);
    if archive_has_files(archive_dir)? {
        state.emit_log(
            "INFO",
            "writing_archive",
            "Skipping writing archive backfill; data/writings already has files",
        );
        return Ok(());
    }

    let writings = {
        let db = state.db.lock().await;
        queries::get_writings_for_file_archive(&db)?
    };

    if writings.is_empty() {
        state.emit_log(
            "INFO",
            "writing_archive",
            "No existing archived writings found for backfill",
        );
        return Ok(());
    }

    state.emit_log(
        "INFO",
        "writing_archive",
        &format!("Backfilling {} writings to data/writings", writings.len()),
    );

    let mut written = 0usize;
    let mut failed = 0usize;

    for writing in writings {
        match sqlite_timestamp_to_unix(&writing.created_at).and_then(|timestamp| {
            save_writing_to_file(&writing.wallet_address, timestamp, &writing.content)
        }) {
            Ok(()) => written += 1,
            Err(e) => {
                failed += 1;
                state.emit_log(
                    "ERROR",
                    "writing_archive",
                    &format!(
                        "Backfill failed for writing {}: {}",
                        &writing.id[..8.min(writing.id.len())],
                        e
                    ),
                );
            }
        }
    }

    state.emit_log(
        "INFO",
        "writing_archive",
        &format!(
            "Writing archive backfill complete: {} written, {} failed",
            written, failed
        ),
    );

    Ok(())
}

pub fn sqlite_timestamp_to_unix(created_at: &str) -> Result<i64> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(created_at) {
        return Ok(parsed.with_timezone(&Utc).timestamp());
    }

    if let Ok(parsed) = NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S%.f") {
        return Ok(parsed.and_utc().timestamp());
    }

    if let Ok(parsed) = NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S") {
        return Ok(parsed.and_utc().timestamp());
    }

    Err(anyhow!(
        "unsupported SQLite timestamp format: {}",
        created_at
    ))
}

fn archive_has_files(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let mut entries =
        fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(entries.next().transpose()?.is_some())
}
