use crate::config::LoggingConfig;
use anyhow::{Context, Result};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static ROTATE_CHECK_COUNTER: AtomicU32 = AtomicU32::new(0);

pub fn append_log(cfg: &LoggingConfig, message: &str) -> Result<()> {
    if let Some(parent) = cfg.file.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory {}", parent.display()))?;
    }

    let check_idx = ROTATE_CHECK_COUNTER.fetch_add(1, Ordering::Relaxed);
    if check_idx.is_multiple_of(64) {
        rotate_if_needed(&cfg.file, cfg.max_bytes, cfg.keep_files)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&cfg.file)
        .with_context(|| format!("failed to open log file {}", cfg.file.display()))?;

    writeln!(file, "{} {}", Utc::now().to_rfc3339(), message)
        .with_context(|| format!("failed to write log file {}", cfg.file.display()))
}

fn rotate_if_needed(path: &Path, max_bytes: u64, keep_files: usize) -> Result<()> {
    if keep_files == 0 || !path.exists() {
        return Ok(());
    }

    let size = fs::metadata(path)
        .with_context(|| format!("failed to read log metadata {}", path.display()))?
        .len();
    if size < max_bytes {
        return Ok(());
    }

    for idx in (1..=keep_files).rev() {
        let src = if idx == 1 {
            path.to_path_buf()
        } else {
            rotated_path(path, idx - 1)
        };
        let dst = rotated_path(path, idx);

        if !src.exists() {
            continue;
        }
        if dst.exists() {
            fs::remove_file(&dst).with_context(|| format!("failed to remove {}", dst.display()))?;
        }
        fs::rename(&src, &dst)
            .with_context(|| format!("failed to rotate {} -> {}", src.display(), dst.display()))?;
    }

    Ok(())
}

fn rotated_path(path: &Path, idx: usize) -> PathBuf {
    PathBuf::from(format!("{}.{}", path.display(), idx))
}
