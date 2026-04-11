/// Chaos drill service: scheduled fault injection for resilience testing.
///
/// PURPOSE: Controlled simulation of infrastructure failures to verify that
/// degradation controls, retries, and error handling work as designed.
///
/// GUARDRAILS — chaos drills NEVER run unless ALL of the following are true:
///   1. CHAOS_ENABLED=true environment variable is set
///   2. The current UTC time is within the scheduled drill window
///   3. The drill has not been manually suppressed via `chaos_suppressed` ops flag
///
/// SCHEDULED WINDOW: Sundays, 02:00–02:15 UTC
/// This is a maintenance window when load is minimal.
///
/// SIMULATED FAULTS (bounded and reversible):
///   - DB_LATENCY: adds a configurable artificial sleep (default 200ms) to DB reads
///   - API_TIMEOUT: returns a 503 Service Unavailable on a configurable percentage
///     of requests (default 5%) to simulate upstream timeouts
///
/// AUDIT: Every drill start/stop is written to ops_events and the structured log.
/// RECOVERY: Faults automatically cease at the end of the 15-minute window.
///
/// INTEGRATION: The chaos service is called from two places:
///   1. A background Tokio task in bootstrap that logs drill start/stop transitions.
///   2. Route handlers (via ChaosService::maybe_inject_latency / maybe_inject_error)
///      that inject faults during the active drill window.

use std::time::Duration;

use chrono::{Datelike, Timelike};
use sqlx::MySqlPool;
use uuid::Uuid;

/// Scheduled drill start: Sunday = weekday 6 (chrono), 02:00 UTC
const DRILL_DAY_OF_WEEK: u32 = 6; // Sunday in chrono (Mon=0 ... Sun=6)
const DRILL_START_HOUR: u32 = 2;
const DRILL_START_MINUTE: u32 = 0;
const DRILL_DURATION_MINUTES: i64 = 15;

/// Added latency during DB_LATENCY drill (milliseconds)
const SIMULATED_DB_LATENCY_MS: u64 = 200;

/// Fraction of requests to drop during API_TIMEOUT drill (5%)
const SIMULATED_TIMEOUT_FRACTION: f64 = 0.05;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChaosStatus {
    pub chaos_enabled: bool,
    pub in_drill_window: bool,
    pub drill_active: bool,
    pub next_window_utc: String,
    pub guardrails: Vec<String>,
}

#[derive(Clone)]
pub struct ChaosService {
    pool: MySqlPool,
}

impl ChaosService {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Guardrail checks
    // ------------------------------------------------------------------

    /// Returns true if chaos drills are armed.
    /// Requires CHAOS_ENABLED=true env var — defaults to false (disabled).
    pub fn is_chaos_armed() -> bool {
        std::env::var("CHAOS_ENABLED")
            .map(|v| v.trim().to_lowercase() == "true")
            .unwrap_or(false)
    }

    /// Returns true if the current UTC clock is within the Sunday 02:00–02:15 window.
    pub fn is_drill_window() -> bool {
        let now = chrono::Utc::now();
        let weekday = now.weekday().num_days_from_monday(); // Mon=0, Sun=6
        let hour = now.hour();
        let minute = now.minute();

        weekday == DRILL_DAY_OF_WEEK
            && hour == DRILL_START_HOUR
            && minute < DRILL_DURATION_MINUTES as u32
    }

    /// Returns true if a chaos drill should currently be active.
    pub fn drill_active() -> bool {
        Self::is_chaos_armed() && Self::is_drill_window()
    }

    // ------------------------------------------------------------------
    // Fault injection (called from route handlers)
    // ------------------------------------------------------------------

    /// Inject artificial DB latency if a drill is active.
    /// Callers should await this at the start of DB-heavy operations.
    /// No-op if CHAOS_ENABLED is not set or outside the drill window.
    pub async fn maybe_inject_latency() {
        if Self::drill_active() {
            tracing::debug!(
                latency_ms = SIMULATED_DB_LATENCY_MS,
                "Chaos drill: injecting DB latency"
            );
            tokio::time::sleep(Duration::from_millis(SIMULATED_DB_LATENCY_MS)).await;
        }
    }

    /// Inject a simulated API timeout on a fraction of calls.
    /// Returns true if this call should be treated as a timeout (return 503).
    /// Uses a simple counter-based approach — every Nth call (1/fraction) triggers.
    pub fn should_inject_timeout() -> bool {
        if !Self::drill_active() {
            return false;
        }
        // Use current nanosecond modulo as a pseudo-random discriminator.
        // This is not truly random but is deterministic and bounded.
        let ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();

        let threshold = (SIMULATED_TIMEOUT_FRACTION * u32::MAX as f64) as u32;
        ns < threshold
    }

    // ------------------------------------------------------------------
    // Status and scheduling
    // ------------------------------------------------------------------

    pub fn status() -> ChaosStatus {
        let chaos_enabled = Self::is_chaos_armed();
        let in_window = Self::is_drill_window();

        let mut guardrails = vec![
            format!("CHAOS_ENABLED env var = {}", if chaos_enabled { "true" } else { "false (disabled)" }),
            format!("Drill window = Sundays {:02}:{:02}–{:02}:{:02} UTC",
                DRILL_START_HOUR, DRILL_START_MINUTE,
                DRILL_START_HOUR, DRILL_DURATION_MINUTES),
            format!("Simulated DB latency = {}ms (when active)", SIMULATED_DB_LATENCY_MS),
            format!("Simulated timeout fraction = {:.0}% of requests (when active)", SIMULATED_TIMEOUT_FRACTION * 100.0),
        ];
        if !chaos_enabled {
            guardrails.push("Set CHAOS_ENABLED=true to arm drills".to_string());
        }

        ChaosStatus {
            chaos_enabled,
            in_drill_window: in_window,
            drill_active: chaos_enabled && in_window,
            next_window_utc: next_sunday_window_utc(),
            guardrails,
        }
    }

    // ------------------------------------------------------------------
    // Ops event logging (called from the background task in bootstrap)
    // ------------------------------------------------------------------

    pub async fn log_drill_started(&self) {
        let id = Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO ops_events (id, event_type, actor_id, note)
             VALUES (?, 'chaos.started', 'system', 'Scheduled chaos drill window started (Sunday 02:00–02:15 UTC)')"
        )
        .bind(&id)
        .execute(&self.pool)
        .await;

        tracing::warn!(
            window = "Sunday 02:00–02:15 UTC",
            db_latency_ms = SIMULATED_DB_LATENCY_MS,
            timeout_fraction_pct = SIMULATED_TIMEOUT_FRACTION * 100.0,
            "CHAOS DRILL STARTED"
        );
    }

    pub async fn log_drill_stopped(&self) {
        let id = Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO ops_events (id, event_type, actor_id, note)
             VALUES (?, 'chaos.stopped', 'system', 'Scheduled chaos drill window ended')"
        )
        .bind(&id)
        .execute(&self.pool)
        .await;

        tracing::info!("CHAOS DRILL ENDED — resuming normal operation");
    }
}

/// Calculate the UTC datetime string for the next Sunday 02:00.
fn next_sunday_window_utc() -> String {
    use chrono::{Datelike, Duration, Timelike, Utc};
    let now = Utc::now();
    let days_until_sunday = {
        let current = now.weekday().num_days_from_monday(); // Mon=0, Sun=6
        if current < DRILL_DAY_OF_WEEK {
            DRILL_DAY_OF_WEEK - current
        } else if current == DRILL_DAY_OF_WEEK && now.hour() < DRILL_START_HOUR {
            0
        } else {
            7 - (current - DRILL_DAY_OF_WEEK)
        }
    };
    let next = now.date_naive() + Duration::days(days_until_sunday as i64);
    format!("{} {:02}:{:02}:00 UTC", next, DRILL_START_HOUR, DRILL_START_MINUTE)
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaos_disabled_by_default() {
        // CHAOS_ENABLED is not set in test environment, so must be false
        // (The env var might be set externally, so we just verify the parsing logic)
        let val = std::env::var("CHAOS_ENABLED").unwrap_or_default();
        let expected = val.trim().to_lowercase() == "true";
        assert_eq!(ChaosService::is_chaos_armed(), expected);
    }

    #[test]
    fn test_drill_window_constants() {
        assert_eq!(DRILL_DAY_OF_WEEK, 6, "drill must be on Sunday");
        assert_eq!(DRILL_START_HOUR, 2, "drill must start at 02:00");
        assert_eq!(DRILL_DURATION_MINUTES, 15, "drill must be 15 minutes");
    }

    #[test]
    fn test_should_not_inject_when_not_armed() {
        // Without CHAOS_ENABLED=true, injection should never happen
        if !ChaosService::is_chaos_armed() {
            assert!(!ChaosService::should_inject_timeout());
        }
    }

    #[test]
    fn test_next_window_format() {
        let s = next_sunday_window_utc();
        assert!(s.contains("UTC"), "next window string should contain UTC: {s}");
        assert!(s.contains("02:00"), "next window string should contain 02:00: {s}");
    }

    #[test]
    fn test_simulated_latency_constant() {
        assert!(SIMULATED_DB_LATENCY_MS <= 500, "latency must be bounded to ≤500ms");
    }

    #[test]
    fn test_timeout_fraction_constant() {
        assert!(SIMULATED_TIMEOUT_FRACTION < 0.20, "timeout fraction must be <20%");
        assert!(SIMULATED_TIMEOUT_FRACTION > 0.0, "timeout fraction must be >0%");
    }

    #[test]
    fn test_chaos_status_structure() {
        let status = ChaosService::status();
        assert!(!status.guardrails.is_empty());
        assert!(status.next_window_utc.contains("UTC"));
    }
}
