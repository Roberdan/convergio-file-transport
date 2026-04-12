//! Domain types for file transport operations.

use serde::{Deserialize, Serialize};

/// Direction of a file transfer between mesh nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    Push,
    Pull,
}

impl std::fmt::Display for TransferDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Push => write!(f, "push"),
            Self::Pull => write!(f, "pull"),
        }
    }
}

/// Request to initiate a file transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub source_path: String,
    pub dest_path: String,
    pub peer_name: String,
    /// SSH target string (e.g. "user@host" or SSH alias).
    pub ssh_target: String,
    pub direction: TransferDirection,
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

impl TransferRequest {
    /// Validate request fields before executing a transfer.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.source_path.trim().is_empty() {
            return Err("source_path must not be empty");
        }
        if self.dest_path.trim().is_empty() {
            return Err("dest_path must not be empty");
        }
        if self.peer_name.trim().is_empty() {
            return Err("peer_name must not be empty");
        }
        if self.ssh_target.trim().is_empty() {
            return Err("ssh_target must not be empty");
        }
        // Reject exclude patterns that look like rsync flags
        for pat in &self.exclude_patterns {
            if pat.starts_with('-') {
                return Err("exclude pattern must not start with '-'");
            }
        }
        Ok(())
    }
}

/// Outcome of an individual transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub peer_name: String,
    pub bytes_transferred: u64,
    pub files_count: u64,
    pub duration_ms: u64,
    pub status: TransferStatus,
}

/// Status of a completed transfer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "message")]
pub enum TransferStatus {
    Success,
    Failed(String),
    PartialSuccess(String),
}

impl std::fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failed(msg) => write!(f, "failed: {msg}"),
            Self::PartialSuccess(msg) => write!(f, "partial: {msg}"),
        }
    }
}

/// Persisted record of a transfer in the DB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    pub id: i64,
    pub peer_name: String,
    pub direction: String,
    pub source_path: String,
    pub dest_path: String,
    pub bytes_transferred: i64,
    pub files_count: i64,
    pub duration_ms: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_display() {
        assert_eq!(TransferDirection::Push.to_string(), "push");
        assert_eq!(TransferDirection::Pull.to_string(), "pull");
    }

    #[test]
    fn status_display() {
        assert_eq!(TransferStatus::Success.to_string(), "success");
        let f = TransferStatus::Failed("timeout".into());
        assert!(f.to_string().contains("timeout"));
    }

    #[test]
    fn request_roundtrip_json() {
        let req = TransferRequest {
            source_path: "/data/project".into(),
            dest_path: "/backup/project".into(),
            peer_name: "studio-mac".into(),
            ssh_target: "rob@studio-mac".into(),
            direction: TransferDirection::Push,
            exclude_patterns: vec!["*.tmp".into()],
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: TransferRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.peer_name, "studio-mac");
        assert_eq!(back.direction, TransferDirection::Push);
    }

    #[test]
    fn validate_rejects_empty_source() {
        let mut req = TransferRequest {
            source_path: "".into(),
            dest_path: "/backup".into(),
            peer_name: "peer".into(),
            ssh_target: "user@host".into(),
            direction: TransferDirection::Push,
            exclude_patterns: vec![],
        };
        assert!(req.validate().is_err());
        req.source_path = "  ".into();
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_fields() {
        let base = TransferRequest {
            source_path: "/src".into(),
            dest_path: "/dst".into(),
            peer_name: "peer".into(),
            ssh_target: "user@host".into(),
            direction: TransferDirection::Push,
            exclude_patterns: vec![],
        };
        assert!(base.validate().is_ok());

        let mut bad = base.clone();
        bad.dest_path = "".into();
        assert!(bad.validate().is_err());

        let mut bad = base.clone();
        bad.peer_name = "".into();
        assert!(bad.validate().is_err());

        let mut bad = base;
        bad.ssh_target = "".into();
        assert!(bad.validate().is_err());
    }

    #[test]
    fn validate_rejects_flag_like_exclude() {
        let req = TransferRequest {
            source_path: "/src".into(),
            dest_path: "/dst".into(),
            peer_name: "peer".into(),
            ssh_target: "user@host".into(),
            direction: TransferDirection::Push,
            exclude_patterns: vec!["--delete-before".into()],
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn direction_is_eq() {
        assert_eq!(TransferDirection::Push, TransferDirection::Push);
        assert_ne!(TransferDirection::Push, TransferDirection::Pull);
    }
}
