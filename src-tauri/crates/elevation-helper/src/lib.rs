use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegedCleanupCapability {
    pub available: bool,
    pub message: String,
}

pub fn privileged_cleanup_capability() -> PrivilegedCleanupCapability {
    PrivilegedCleanupCapability {
        available: false,
        message: "Protected-path cleanup stays outside the unprivileged desktop flow in this milestone.".to_string(),
    }
}

pub fn requires_elevation(path: &str) -> bool {
    let normalized = normalize_path(path);
    if normalized.is_empty() {
        return false;
    }

    protected_prefixes()
        .into_iter()
        .any(|prefix| normalized == prefix || normalized.starts_with(&format!("{prefix}\\")))
}

fn protected_prefixes() -> Vec<String> {
    let mut prefixes = Vec::new();

    for value in [
        std::env::var("windir").ok(),
        std::env::var("ProgramFiles").ok(),
        std::env::var("ProgramFiles(x86)").ok(),
    ]
    .into_iter()
    .flatten()
    {
        let normalized = normalize_path(&value);
        if !normalized.is_empty() && !prefixes.contains(&normalized) {
            prefixes.push(normalized);
        }
    }

    for fallback in [
        r"C:\Windows",
        r"C:\Program Files",
        r"C:\Program Files (x86)",
    ] {
        let normalized = normalize_path(fallback);
        if !prefixes.contains(&normalized) {
            prefixes.push(normalized);
        }
    }

    prefixes
}

fn normalize_path(value: &str) -> String {
    value.trim().trim_end_matches(['\\', '/']).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_profile_path_does_not_require_elevation() {
        assert!(!requires_elevation(r"C:\Users\xiongxianfei\Downloads\left.bin"));
    }

    #[test]
    fn protected_windows_paths_require_elevation() {
        assert!(requires_elevation(r"C:\Windows\Temp\cache.tmp"));
        assert!(requires_elevation(r"C:\Program Files\Space Sift\cache.tmp"));
    }

    #[test]
    fn capability_stays_fail_closed() {
        let capability = privileged_cleanup_capability();
        assert!(!capability.available);
        assert!(capability.message.contains("unprivileged"));
    }
}
