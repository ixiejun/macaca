//! Logging initialization module with file output, rotation, and compression.
//!
//! This module provides a production-grade logging system that:
//! - Writes logs to both console and file
//! - Rotates log files daily (using LOCAL timezone)
//! - Compresses old log files (previous days)
//! - Cleans up logs older than retention period

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use chrono::{Local, NaiveDate};
use tracing::info;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

use macaca_proto::config::LogFileConfig;

/// Global guard for the non-blocking file writer.
/// Must be kept alive for the duration of the application.
static mut FILE_APPENDER_GUARD: Option<tracing_appender::non_blocking::WorkerGuard> = None;

/// A daily rolling file appender that uses LOCAL timezone for date rotation.
pub struct LocalDailyRollingAppender {
    dir: PathBuf,
    prefix: String,
    current_file: Option<(File, NaiveDate)>,
}

impl LocalDailyRollingAppender {
    pub fn new(dir: impl Into<PathBuf>, prefix: impl Into<String>) -> Self {
        Self {
            dir: dir.into(),
            prefix: prefix.into(),
            current_file: None,
        }
    }

    fn get_current_date() -> NaiveDate {
        Local::now().date_naive()
    }

    fn get_file_path(&self, date: NaiveDate) -> PathBuf {
        self.dir
            .join(format!("{}.{}", self.prefix, date.format("%Y-%m-%d")))
    }

    fn ensure_file(&mut self) -> io::Result<&File> {
        let today = Self::get_current_date();

        // Check if we need to create or rotate the file
        match &self.current_file {
            Some((_, file_date)) if *file_date == today => {
                // Same day, file is still valid
            }
            _ => {
                // Need to create new file for today
                let path = self.get_file_path(today);
                let file = File::options().create(true).append(true).open(&path)?;
                self.current_file = Some((file, today));
            }
        }

        Ok(&self.current_file.as_ref().unwrap().0)
    }
}

impl io::Write for LocalDailyRollingAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.ensure_file()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some((file, _)) = &mut self.current_file {
            file.flush()?;
        }
        Ok(())
    }
}

/// Wrapper to make a Mutex<Write> implement io::Write for tracing_appender.
struct MutexWriter<W>(Mutex<W>);

impl<W: io::Write> io::Write for MutexWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

/// Initialize the logging system with both console and file output.
///
/// # Arguments
/// * `config` - Log file configuration
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(...)` on failure
pub fn init_logging(
    config: &LogFileConfig,
    log_level: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create log directory if it doesn't exist
    if config.enabled {
        fs::create_dir_all(&config.dir)?;
    }

    // Build the env filter: RUST_LOG env var takes priority, then config log_level
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    if config.enabled {
        // Create local timezone daily rolling file appender
        let file_appender = LocalDailyRollingAppender::new(&config.dir, &config.prefix);

        // Wrap in Mutex for thread safety
        let file_appender = Mutex::new(file_appender);

        // Create non-blocking writer for performance
        let (non_blocking, guard) = tracing_appender::non_blocking(MutexWriter(file_appender));

        // Store guard globally to keep it alive
        // SAFETY: This is called once during initialization
        unsafe {
            FILE_APPENDER_GUARD = Some(guard);
        }

        // Choose format based on config
        let file_layer = if config.format == "json" {
            fmt::layer()
                .json()
                .with_span_events(FmtSpan::CLOSE)
                .with_writer(non_blocking)
                .with_ansi(false)
                .boxed()
        } else {
            fmt::layer()
                .with_span_events(FmtSpan::CLOSE)
                .with_writer(non_blocking)
                .with_ansi(false)
                .boxed()
        };

        // Console layer (always pretty format for readability)
        let console_layer = fmt::layer()
            .pretty()
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .boxed();

        // Initialize subscriber with both layers
        tracing_subscriber::registry()
            .with(filter)
            .with(console_layer)
            .with(file_layer)
            .init();

        info!(
            dir = %config.dir,
            prefix = %config.prefix,
            format = %config.format,
            retention_days = config.retention_days,
            compress = config.compress,
            "Logging system initialized with file output"
        );

        // Spawn background task for log cleanup and compression
        if config.compress || config.retention_days > 0 {
            spawn_log_cleanup_task(config.clone());
        }
    } else {
        // Console-only logging
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .pretty()
            .init();

        info!("Logging system initialized (console only)");
    }

    Ok(())
}

/// Spawn a background task to handle log cleanup and compression.
fn spawn_log_cleanup_task(config: LogFileConfig) {
    tokio::spawn(async move {
        // Run cleanup immediately on startup
        if let Err(e) = cleanup_old_logs(&config).await {
            tracing::warn!(error = %e, "Failed to cleanup old logs on startup");
        }

        // Then run every 24 hours
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));

        loop {
            interval.tick().await;

            if let Err(e) = cleanup_old_logs(&config).await {
                tracing::warn!(error = %e, "Failed to cleanup old logs");
            }
        }
    });
}

/// Clean up old log files: compress previous days and delete files past retention.
async fn cleanup_old_logs(
    config: &LogFileConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let log_dir = Path::new(&config.dir);

    if !log_dir.exists() {
        return Ok(());
    }

    let today = Local::now().date_naive();
    let retention_threshold = today - chrono::Duration::days(config.retention_days as i64);

    let entries: Vec<_> = fs::read_dir(log_dir)?.filter_map(|e| e.ok()).collect();

    for entry in entries {
        let path = entry.path();

        // Only process regular files
        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(f) => f,
            None => continue,
        };

        // Check if it's a log file (matches prefix)
        if !filename.starts_with(&config.prefix) {
            continue;
        }

        // Skip already compressed files
        if filename.ends_with(".gz") {
            // Check retention for compressed files
            if should_delete_compressed(&filename, retention_threshold)? {
                fs::remove_file(&path)?;
                info!(file = %filename, "Deleted old compressed log file");
            }
            continue;
        }

        // Try to extract date from filename (format: prefix.YYYY-MM-DD)
        if let Some(file_date) = extract_date_from_filename(filename) {
            // Skip today's log
            if file_date == today {
                continue;
            }

            // Delete if past retention period
            if file_date < retention_threshold {
                fs::remove_file(&path)?;
                info!(file = %filename, date = %file_date, "Deleted old log file (retention)");
                continue;
            }

            // Compress if enabled and not today
            if config.compress {
                let compressed_path = format!("{}.gz", path.display());
                compress_file(&path, &compressed_path)?;
                fs::remove_file(&path)?;
                info!(file = %filename, "Compressed log file");
            }
        }
    }

    Ok(())
}

/// Extract date from log filename (format: prefix.YYYY-MM-DD).
fn extract_date_from_filename(filename: &str) -> Option<NaiveDate> {
    // Try to find date pattern YYYY-MM-DD in filename
    let parts: Vec<&str> = filename.split('.').collect();

    for part in parts {
        if let Ok(date) = NaiveDate::parse_from_str(part, "%Y-%m-%d") {
            return Some(date);
        }
    }

    None
}

/// Check if a compressed log file should be deleted based on retention.
fn should_delete_compressed(
    filename: &str,
    retention_threshold: NaiveDate,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    // Remove .gz suffix and try to extract date
    let without_gz = filename.strip_suffix(".gz").unwrap_or(filename);

    if let Some(file_date) = extract_date_from_filename(without_gz) {
        return Ok(file_date < retention_threshold);
    }

    Ok(false)
}

/// Compress a file using gzip.
fn compress_file(
    source: &Path,
    dest: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::fs::File;
    use std::io::{BufReader, BufWriter};

    let input = File::open(source)?;
    let output = File::create(dest)?;

    let mut reader = BufReader::new(input);
    let mut writer = BufWriter::new(output);

    // Use flate2 for gzip compression
    let mut encoder = flate2::write::GzEncoder::new(&mut writer, flate2::Compression::default());
    std::io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;

    Ok(())
}
