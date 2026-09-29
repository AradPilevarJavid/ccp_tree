use anyhow::{Context, Result};
use serde::Deserialize;
use std::sync::mpsc;
use std::time::Duration;

const CRATES_IO_API_URL: &str = "https://crates.io/api/v1/crates/ccp_tree";
const CHECK_TIMEOUT: Duration = Duration::from_millis(100);
const REQUEST_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    package: CrateInfo,
}

#[derive(Debug, Deserialize)]
struct CrateInfo {
    max_version: String,
}

pub fn notify_if_available() {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(latest_version().ok().and_then(|latest| {
            if is_newer_version(env!("CARGO_PKG_VERSION"), &latest) {
                Some(format_notification(env!("CARGO_PKG_VERSION"), &latest))
            } else {
                None
            }
        }));
    });

    if let Ok(Some(message)) = receiver.recv_timeout(CHECK_TIMEOUT) {
        eprintln!("{message}");
    }
}

pub fn latest_version() -> Result<String> {
    let response: CratesIoResponse = ureq::get(CRATES_IO_API_URL)
        .set(
            "User-Agent",
            concat!("ccp_tree/", env!("CARGO_PKG_VERSION")),
        )
        .timeout(REQUEST_TIMEOUT)
        .call()
        .context("failed to query crates.io")?
        .into_json()
        .context("failed to parse crates.io response")?;
    Ok(response.package.max_version)
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let Ok(current) = semver::Version::parse(current) else {
        return false;
    };
    let Ok(latest) = semver::Version::parse(latest) else {
        return false;
    };
    latest > current
}

pub fn format_notification(current: &str, latest: &str) -> String {
    format!(
        "A new version of ccp_tree is available: {current} → {latest}\n\
         Run `ccp update` to install it."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_newer_semver() {
        assert!(is_newer_version("1.1.2", "1.1.3"));
        assert!(!is_newer_version("1.1.3", "1.1.2"));
        assert!(!is_newer_version("invalid", "1.1.3"));
    }

    #[test]
    fn formats_requested_notification() {
        assert_eq!(
            format_notification("1.0.3", "1.0.4"),
            "A new version of ccp_tree is available: 1.0.3 → 1.0.4\n\
             Run `ccp update` to install it."
        );
    }
}
