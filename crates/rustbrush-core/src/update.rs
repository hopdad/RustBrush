//! Background update checker — queries GitHub Releases API on startup.
//!
//! Spawns a background thread that checks for a newer release.
//! If found, returns an `UpdateInfo` via an `mpsc::Receiver`.
//! All errors are silently ignored.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Information about an available update.
pub struct UpdateInfo {
    pub latest_version: String,
    pub current_version: String,
    pub release_url: String,
}

/// Spawns a background thread to check GitHub for a newer release.
///
/// Returns a `Receiver` that will yield `UpdateInfo` if an update is available.
/// If the check fails or no update is found, the channel simply closes.
pub fn check_for_update() -> mpsc::Receiver<UpdateInfo> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = check_and_notify(tx);
    });
    rx
}

fn check_and_notify(tx: mpsc::Sender<UpdateInfo>) -> Result<(), Box<dyn std::error::Error>> {
    let current = env!("CARGO_PKG_VERSION");

    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .new_agent();

    let body: String = agent
        .get("https://api.github.com/repos/hopdad/RustBrush/releases/latest")
        .header("User-Agent", "RustBrush-UpdateCheck")
        .header("Accept", "application/vnd.github.v3+json")
        .call()?
        .body_mut()
        .read_to_string()?;

    let json: serde_json::Value = serde_json::from_str(&body)?;

    let tag = json["tag_name"]
        .as_str()
        .ok_or("missing tag_name")?
        .trim_start_matches('v');

    let html_url = json["html_url"]
        .as_str()
        .unwrap_or("https://github.com/hopdad/RustBrush/releases")
        .to_string();

    if is_newer(tag, current) {
        let _ = tx.send(UpdateInfo {
            latest_version: tag.to_string(),
            current_version: current.to_string(),
            release_url: html_url,
        });
    }

    Ok(())
}

/// Returns true if `remote` is strictly newer than `local` (semver comparison).
fn is_newer(remote: &str, local: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .filter_map(|p| p.parse::<u64>().ok())
            .collect()
    };
    parse(remote) > parse(local)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.3.0", "0.2.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.2.1", "0.2.0"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }
}
