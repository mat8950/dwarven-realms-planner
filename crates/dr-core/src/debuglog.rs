//! Append-only debug journal shared by the app and the HTTP bridge:
//! `~/.cache/dr-planner/app.log` (rotated to `app.log.1` past 5 MB).
//! Each line: `<UTC time> <LEVEL> [<source>] <message>`.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static LOCK: Mutex<()> = Mutex::new(());
const MAX_BYTES: u64 = 5_000_000;

pub fn path() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache"));
    base.join("dr-planner/app.log")
}

/// `2026-10-28 15:20:03.123Z`
pub fn timestamp() -> String {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let (secs, ms) = (d.as_secs() as i64, d.subsec_millis());
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}.{ms:03}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

pub fn write(level: &str, source: &str, message: &str) {
    let _guard = LOCK.lock();
    let p = path();
    if let Some(dir) = p.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if fs::metadata(&p).map(|m| m.len() > MAX_BYTES).unwrap_or(false) {
        let _ = fs::rename(&p, p.with_extension("log.1"));
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&p) {
        // One entry per line: indent continuation lines of multi-line messages.
        let _ = writeln!(f, "{} {level:<5} [{source}] {}", timestamp(), message.replace('\n', "\n    "));
    }
}

pub fn info(source: &str, message: &str) {
    write("INFO", source, message);
}

pub fn error(source: &str, message: &str) {
    write("ERROR", source, message);
}

/// Last `n` lines of the journal.
pub fn tail(n: usize) -> Vec<String> {
    let text = fs::read_to_string(path()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].iter().map(|s| s.to_string()).collect()
}

/// Logs panics (with location) before the default hook prints them.
pub fn install_panic_hook(source: &'static str) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
        let at = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        write("PANIC", source, &format!("{msg}{at}"));
        previous(info);
    }));
}
