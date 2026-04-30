//! Rate-limit and credits display shaping for lightweight status surfaces.
//!
//! This module maps `RateLimitSnapshot` protocol payloads into display-oriented values used by
//! footer/status-line contexts and internal refresh state. `/status` intentionally does not render
//! Codex/ChatGPT account limits because DeepSeek API usage does not have that quota surface.
//!
//! Reset timestamps stay in the protocol snapshot for warning logic, but this lightweight display
//! adapter only keeps values the current footer/status-line consumers render.
use chrono::DateTime;
use chrono::Local;
use codex_protocol::protocol::CreditsSnapshot as CoreCreditsSnapshot;
use codex_protocol::protocol::RateLimitSnapshot;
use codex_protocol::protocol::RateLimitWindow;

/// Display-friendly representation of one usage window from a snapshot.
#[derive(Debug, Clone)]
pub(crate) struct RateLimitWindowDisplay {
    /// Percent used for the window.
    pub used_percent: f64,
    /// Window length in minutes when provided by the server.
    pub window_minutes: Option<i64>,
}

impl RateLimitWindowDisplay {
    fn from_window(window: &RateLimitWindow) -> Self {
        Self {
            used_percent: window.used_percent,
            window_minutes: window.window_minutes,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RateLimitSnapshotDisplay {
    /// Primary usage window (typically short duration).
    pub primary: Option<RateLimitWindowDisplay>,
    /// Secondary usage window (typically weekly).
    pub secondary: Option<RateLimitWindowDisplay>,
    /// Optional credits metadata when available.
    pub credits: Option<CreditsSnapshotDisplay>,
}

/// Display-ready credits state extracted from protocol snapshots.
#[derive(Debug, Clone)]
pub(crate) struct CreditsSnapshotDisplay {
    /// Whether credits tracking is enabled for the account.
    pub has_credits: bool,
    /// Whether the account has unlimited credits.
    pub unlimited: bool,
    /// Raw balance text as provided by the backend.
    pub balance: Option<String>,
}

/// Converts a protocol snapshot into UI-friendly display data.
///
/// The capture timestamp remains in the API so callers do not need separate conversion paths for
/// surfaces that still observe rate-limit snapshot freshness.
#[cfg(test)]
pub(crate) fn rate_limit_snapshot_display(
    snapshot: &RateLimitSnapshot,
    captured_at: DateTime<Local>,
) -> RateLimitSnapshotDisplay {
    rate_limit_snapshot_display_for_limit(snapshot, "codex".to_string(), captured_at)
}

pub(crate) fn rate_limit_snapshot_display_for_limit(
    snapshot: &RateLimitSnapshot,
    _limit_name: String,
    _captured_at: DateTime<Local>,
) -> RateLimitSnapshotDisplay {
    RateLimitSnapshotDisplay {
        primary: snapshot
            .primary
            .as_ref()
            .map(RateLimitWindowDisplay::from_window),
        secondary: snapshot
            .secondary
            .as_ref()
            .map(RateLimitWindowDisplay::from_window),
        credits: snapshot.credits.as_ref().map(CreditsSnapshotDisplay::from),
    }
}

impl From<&CoreCreditsSnapshot> for CreditsSnapshotDisplay {
    fn from(value: &CoreCreditsSnapshot) -> Self {
        Self {
            has_credits: value.has_credits,
            unlimited: value.unlimited,
            balance: value.balance.clone(),
        }
    }
}
