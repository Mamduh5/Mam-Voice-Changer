use std::{path::Path, process::Command};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckoutCleanliness {
    Clean,
    Dirty,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackendRepositoryInspection {
    pub checkout_label: String,
    pub git_directory_present: bool,
    pub git_available: bool,
    pub remote_identity: Option<String>,
    pub commit_sha: Option<String>,
    pub detached_head: Option<bool>,
    pub cleanliness: CheckoutCleanliness,
    pub tracked_changes: u32,
    pub untracked_adapter_files: u32,
    pub warnings: Vec<String>,
}

pub fn inspect_repository(checkout: &Path) -> BackendRepositoryInspection {
    let mut inspection = BackendRepositoryInspection {
        checkout_label: checkout
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("configured-checkout")
            .to_owned(),
        git_directory_present: checkout.join(".git").exists(),
        git_available: false,
        remote_identity: None,
        commit_sha: None,
        detached_head: None,
        cleanliness: CheckoutCleanliness::Unknown,
        tracked_changes: 0,
        untracked_adapter_files: 0,
        warnings: Vec::new(),
    };
    if !inspection.git_directory_present {
        inspection.warnings.push(
            "The configured checkout has no .git metadata; revision identity is unavailable."
                .to_owned(),
        );
        return inspection;
    }

    let head = fixed_git(checkout, &["rev-parse", "HEAD"]);
    inspection.git_available = head.is_some();
    inspection.commit_sha = head
        .filter(|value| value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if !inspection.git_available {
        inspection
            .warnings
            .push("Git is unavailable; the environment cannot be fully reproducible.".to_owned());
        return inspection;
    }
    inspection.remote_identity = fixed_git(checkout, &["remote", "get-url", "origin"])
        .map(|remote| sanitize_remote_url(&remote));
    inspection.detached_head = fixed_git(checkout, &["symbolic-ref", "-q", "HEAD"])
        .map(|value| value.is_empty())
        .or(Some(true));
    if let Some(status) = fixed_git(checkout, &["status", "--porcelain"]) {
        let (tracked, untracked) = classify_status(&status);
        inspection.tracked_changes = tracked;
        inspection.untracked_adapter_files = untracked;
        inspection.cleanliness = if tracked == 0 && untracked == 0 {
            CheckoutCleanliness::Clean
        } else {
            CheckoutCleanliness::Dirty
        };
        if inspection.cleanliness == CheckoutCleanliness::Dirty {
            inspection.warnings.push(
                "The configured backend checkout is dirty; qualification is not reproducible."
                    .to_owned(),
            );
        }
    }
    if inspection.commit_sha.is_none() {
        inspection
            .warnings
            .push("The exact backend commit SHA is unknown.".to_owned());
    }
    inspection
}

fn fixed_git(checkout: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(checkout)
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.len() > 64 * 1024 {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub fn sanitize_remote_url(remote: &str) -> String {
    let trimmed = remote.trim();
    if let Some(scheme) = trimmed.find("://") {
        let prefix = &trimmed[..scheme + 3];
        let rest = &trimmed[scheme + 3..];
        let without_user = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
        let end = without_user.find(['?', '#']).unwrap_or(without_user.len());
        return format!("{prefix}{}", &without_user[..end]);
    }
    let end = trimmed.find(['?', '#']).unwrap_or(trimmed.len());
    let value = &trimmed[..end];
    // SCP-like Git remotes can contain a username, but no password/token is needed.
    value
        .rsplit_once('@')
        .map_or(value, |(_, host)| host)
        .to_owned()
}

fn classify_status(status: &str) -> (u32, u32) {
    let mut tracked = 0;
    let mut untracked = 0;
    for line in status.lines().filter(|line| !line.trim().is_empty()) {
        if line.starts_with("??") {
            let path = line.get(3..).unwrap_or_default().to_ascii_lowercase();
            if path.ends_with(".py")
                || path.ends_with(".yaml")
                || path.ends_with(".yml")
                || path.ends_with(".json")
            {
                untracked += 1;
            }
        } else {
            tracked += 1;
        }
    }
    (tracked, untracked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_credentials_tokens_queries_and_scp_usernames() {
        assert_eq!(
            sanitize_remote_url("https://user:token@example.test/org/repo.git?token=secret#x"),
            "https://example.test/org/repo.git"
        );
        assert_eq!(
            sanitize_remote_url("git@example.test:org/repo.git"),
            "example.test:org/repo.git"
        );
    }

    #[test]
    fn classifies_clean_dirty_and_adapter_relevant_untracked_files() {
        assert_eq!(classify_status(""), (0, 0));
        assert_eq!(
            classify_status(" M train.py\n?? adapter.py\n?? notes.txt\n"),
            (1, 1)
        );
    }

    #[test]
    fn missing_git_metadata_is_nonfatal_and_unknown() {
        let root = std::env::temp_dir().join("mam-voice-missing-git");
        let _ = std::fs::create_dir_all(&root);
        let report = inspect_repository(&root);
        assert!(!report.git_directory_present);
        assert_eq!(report.cleanliness, CheckoutCleanliness::Unknown);
        let _ = std::fs::remove_dir_all(root);
    }
}
