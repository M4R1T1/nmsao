use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::SystemTime;
use std::fs::OpenOptions;
use std::io::Write;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

// sets global log file path
pub(crate) fn init_logging(path: PathBuf) {
    let _ = std::fs::remove_file(&path);
    LOG_PATH.set(path).expect("Log path already set");
    log_message("=== NMSAO Log ===");
}

// timestamp for log file
pub(crate) fn log_message(msg: &str) {
    let path = match LOG_PATH.get() {
        Some(p) => p,
        None => return,
    };
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| format!("{:.3}", d.as_secs_f64()))
        .unwrap_or_else(|_| "?".to_string());
    let line = format!("[{}] {}\n", timestamp, msg);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(line.as_bytes());
    }
}