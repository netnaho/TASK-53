/// Operational Resilience & Observability page.
///
/// Tabs:
///   Health     — /api/health/ready + /api/health/live status
///   Metrics    — sliding-window error rate, request counts
///   Alerts     — current alarm state (badge if ALERTING)
///   Toggles    — degradation flag controls (api.ops.write required to toggle)
///   Chaos      — chaos drill schedule and guardrail info (read-only)

use dioxus::prelude::*;

use crate::state::AuthState;

#[derive(Debug, Clone, PartialEq)]
enum OpsTab {
    Health,
    Metrics,
    Alerts,
    Toggles,
    Chaos,
}

#[component]
pub fn Ops() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut active_tab = use_signal(|| OpsTab::Health);

    let can_read_ops = auth.read().has_permission("api.ops.read");
    let can_write_ops = auth.read().has_permission("api.ops.write");

    if !can_read_ops {
        return rsx! {
            div { class: "page-header",
                h1 { "Operational Controls" }
                p { style: "color: var(--color-text-secondary);",
                    "You do not have permission to view operational controls (api.ops.read required)."
                }
            }
        };
    }

    rsx! {
        div {
            div { class: "page-header",
                h1 { "Operational Controls" }
                p { "System health, error-rate metrics, active alarms, degradation toggles, and chaos drill status." }
            }

            // Tab navigation
            div { style: "display: flex; gap: 4px; border-bottom: 2px solid var(--color-border); margin-bottom: 24px; flex-wrap: wrap;",
                OpsTabButton {
                    label: "Health",
                    active: *active_tab.read() == OpsTab::Health,
                    onclick: move |_| { *active_tab.write() = OpsTab::Health; }
                }
                OpsTabButton {
                    label: "Metrics",
                    active: *active_tab.read() == OpsTab::Metrics,
                    onclick: move |_| { *active_tab.write() = OpsTab::Metrics; }
                }
                OpsTabButton {
                    label: "Alerts",
                    active: *active_tab.read() == OpsTab::Alerts,
                    onclick: move |_| { *active_tab.write() = OpsTab::Alerts; }
                }
                OpsTabButton {
                    label: "Toggles",
                    active: *active_tab.read() == OpsTab::Toggles,
                    onclick: move |_| { *active_tab.write() = OpsTab::Toggles; }
                }
                OpsTabButton {
                    label: "Chaos Drill",
                    active: *active_tab.read() == OpsTab::Chaos,
                    onclick: move |_| { *active_tab.write() = OpsTab::Chaos; }
                }
            }

            match *active_tab.read() {
                OpsTab::Health   => rsx! { HealthTab {} },
                OpsTab::Metrics  => rsx! { MetricsTab {} },
                OpsTab::Alerts   => rsx! { AlertsTab {} },
                OpsTab::Toggles  => rsx! { TogglesTab { can_write: can_write_ops } },
                OpsTab::Chaos    => rsx! { ChaosTab {} },
            }
        }
    }
}

// ============================================================
// Health tab
// ============================================================

#[component]
fn HealthTab() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 16px;",

            // Live probe
            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Liveness Probe" }
                p { style: "font-size: 0.875rem; color: var(--color-text-secondary); margin-bottom: 8px;",
                    "Endpoint: "
                    code { "GET /api/health/live" }
                }
                p { style: "font-size: 0.875rem;",
                    "Returns 200 OK as long as the process is running. Used by container orchestrators to determine if the process should be restarted."
                }
                StatusBadge { label: "Liveness", status: "ok" }
            }

            // Ready probe
            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Readiness Probe" }
                p { style: "font-size: 0.875rem; color: var(--color-text-secondary); margin-bottom: 8px;",
                    "Endpoint: "
                    code { "GET /api/health/ready" }
                }
                p { style: "font-size: 0.875rem; margin-bottom: 12px;",
                    "Executes a lightweight DB ping. Returns 200 when the database is reachable. Used by load balancers to determine if traffic should be routed here."
                }
                div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px;",
                    InfoCard { title: "db_ok", value: "true", note: "MySQL SELECT 1 ping" }
                    InfoCard { title: "chaos_active", value: "false", note: "CHAOS_ENABLED=false (default)" }
                    InfoCard { title: "status", value: "ok", note: "All readiness probes pass" }
                }
            }

            // Endpoints summary
            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Health Endpoints" }
                table { style: "width: 100%; border-collapse: collapse; font-size: 0.875rem;",
                    thead {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            th { style: "padding: 8px 12px; text-align: left;", "Endpoint" }
                            th { style: "padding: 8px 12px; text-align: left;", "Purpose" }
                            th { style: "padding: 8px 12px; text-align: left;", "Auth Required" }
                        }
                    }
                    tbody {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            td { style: "padding: 8px 12px;", code { "GET /api/health/live" } }
                            td { style: "padding: 8px 12px;", "Process alive check" }
                            td { style: "padding: 8px 12px;", "No" }
                        }
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            td { style: "padding: 8px 12px;", code { "GET /api/health/ready" } }
                            td { style: "padding: 8px 12px;", "DB connectivity check" }
                            td { style: "padding: 8px 12px;", "No" }
                        }
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            td { style: "padding: 8px 12px;", code { "GET /api/health/metrics" } }
                            td { style: "padding: 8px 12px;", "Error-rate metrics snapshot" }
                            td { style: "padding: 8px 12px;", "Yes (Bearer + api.ops.read)" }
                        }
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            td { style: "padding: 8px 12px;", code { "GET /api/health/alerts" } }
                            td { style: "padding: 8px 12px;", "Current alarm state" }
                            td { style: "padding: 8px 12px;", "Yes (Bearer + api.ops.read)" }
                        }
                        tr {
                            td { style: "padding: 8px 12px;", code { "GET /api/health/chaos" } }
                            td { style: "padding: 8px 12px;", "Chaos drill status" }
                            td { style: "padding: 8px 12px;", "Yes (Bearer + api.ops.read)" }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// Metrics tab
// ============================================================

#[component]
fn MetricsTab() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 16px;",

            div { class: "card",
                h3 { style: "margin-bottom: 4px;", "In-Process Error Rate Metrics" }
                p { style: "font-size: 0.875rem; color: var(--color-text-secondary); margin-bottom: 16px;",
                    "Sliding 10-minute window. Counts HTTP responses with status ≥ 500 as errors. "
                    "No external dependencies — metrics are stored purely in process memory."
                }

                div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 16px; margin-bottom: 16px;",
                    MetricCard {
                        title: "Window Error Rate",
                        value: "—",
                        unit: "%",
                        description: "5xx responses / total (last 10 min)",
                        highlight: false
                    }
                    MetricCard {
                        title: "Window Requests",
                        value: "—",
                        unit: "req",
                        description: "Requests in 10-min window",
                        highlight: false
                    }
                    MetricCard {
                        title: "Window Errors",
                        value: "—",
                        unit: "err",
                        description: "5xx responses in window",
                        highlight: false
                    }
                    MetricCard {
                        title: "Total Requests",
                        value: "—",
                        unit: "all-time",
                        description: "Since last process restart",
                        highlight: false
                    }
                }

                div { style: "background: var(--color-bg-secondary, #f8f9fa); border-radius: 6px; padding: 12px 16px; font-size: 0.8125rem;",
                    p { style: "font-weight: 600; margin-bottom: 6px;", "Alert Rule" }
                    p {
                        "An alarm transitions to "
                        strong { "ALERTING" }
                        " when the 10-minute window error rate exceeds "
                        strong { "2.0%" }
                        " (strictly greater than). "
                        "Transitions back to "
                        strong { "OK" }
                        " once the rate drops to 2.0% or below."
                    }
                    p { style: "margin-top: 8px; color: var(--color-text-secondary);",
                        "Each state transition is written to the "
                        code { "ops_events" }
                        " table and emitted as a structured log entry."
                    }
                }
            }

            // How to query
            div { class: "card",
                h3 { style: "margin-bottom: 8px;", "Metrics API" }
                p { style: "font-size: 0.875rem; margin-bottom: 4px;",
                    "Query the live snapshot via: "
                    code { "GET /api/health/metrics" }
                }
                p { style: "font-size: 0.875rem; color: var(--color-text-secondary);",
                    "Returns "
                    code { "total_requests" }
                    ", "
                    code { "total_errors" }
                    ", "
                    code { "window_requests" }
                    ", "
                    code { "window_errors" }
                    ", "
                    code { "window_error_rate_pct" }
                    ", "
                    code { "alert_rule" }
                    ", "
                    code { "threshold_pct" }
                    "."
                }
            }
        }
    }
}

#[component]
fn MetricCard(
    title: &'static str,
    value: &'static str,
    unit: &'static str,
    description: &'static str,
    highlight: bool,
) -> Element {
    let border = if highlight { "border-left: 3px solid var(--color-danger, #ef4444);" } else { "" };
    rsx! {
        div { class: "card", style: "padding: 14px 16px; {border}",
            p { style: "font-size: 0.75rem; color: var(--color-text-secondary); margin-bottom: 4px;",
                "{title}"
            }
            p { style: "font-size: 1.75rem; font-weight: 700; color: var(--color-primary); margin-bottom: 2px;",
                "{value}"
                span { style: "font-size: 0.75rem; font-weight: 400; color: var(--color-text-secondary); margin-left: 4px;",
                    "{unit}"
                }
            }
            p { style: "font-size: 0.75rem; color: var(--color-text-secondary);",
                "{description}"
            }
        }
    }
}

// ============================================================
// Alerts tab
// ============================================================

#[component]
fn AlertsTab() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 16px;",

            // Current alarm state card
            div { class: "card",
                div { style: "display: flex; align-items: center; gap: 12px; margin-bottom: 16px;",
                    h3 { style: "margin: 0;", "Current Alarm State" }
                    // Static placeholder badge — would be driven by live data in a full SPA
                    span { style: "padding: 3px 10px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; background: #dcfce7; color: #166534;",
                        "OK"
                    }
                }

                div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; margin-bottom: 16px;",
                    InfoCard { title: "Status", value: "ok", note: "No alarm active" }
                    InfoCard { title: "Current Error Rate", value: "—", note: "From /api/health/alerts" }
                    InfoCard { title: "Window Requests", value: "—", note: "10-minute sliding window" }
                    InfoCard { title: "Alarm Since", value: "—", note: "Unix timestamp of last transition" }
                }

                div { style: "background: var(--color-bg-secondary, #f8f9fa); border-radius: 6px; padding: 12px 16px; font-size: 0.8125rem;",
                    p { style: "font-weight: 600; margin-bottom: 6px;", "Edge-Triggered Alarm Logic" }
                    ul { style: "margin: 0; padding-left: 20px; display: flex; flex-direction: column; gap: 4px;",
                        li { "Evaluated every 30 seconds by a background Tokio task." }
                        li { "DB write to "
                            code { "ops_events" }
                            " only occurs on state transitions (OK → ALERTING or ALERTING → OK)."
                        }
                        li { "Transition to ALERTING: window error rate "
                            strong { "strictly > 2%" }
                        }
                        li { "Transition to OK: window error rate ≤ 2%." }
                    }
                }
            }

            // Ops events log note
            div { class: "card",
                h3 { style: "margin-bottom: 8px;", "ops_events Table" }
                p { style: "font-size: 0.875rem; margin-bottom: 8px;",
                    "All alarm transitions, toggle changes, and chaos drill start/stop events are written to the "
                    code { "ops_events" }
                    " immutable log table."
                }
                table { style: "width: 100%; border-collapse: collapse; font-size: 0.875rem;",
                    thead {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            th { style: "padding: 8px 12px; text-align: left;", "Column" }
                            th { style: "padding: 8px 12px; text-align: left;", "Description" }
                        }
                    }
                    tbody {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            td { style: "padding: 8px 12px;", code { "event_type" } }
                            td { style: "padding: 8px 12px;", "alarm_ok, alarm_alerting, toggle_change, chaos_drill_started, chaos_drill_stopped" }
                        }
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            td { style: "padding: 8px 12px;", code { "key_name" } }
                            td { style: "padding: 8px 12px;", "Toggle key (e.g. exports_enabled) or alarm identifier" }
                        }
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            td { style: "padding: 8px 12px;", code { "old_value / new_value" } }
                            td { style: "padding: 8px 12px;", "Before/after for toggles; error-rate % for alarms" }
                        }
                        tr {
                            td { style: "padding: 8px 12px;", code { "actor_id" } }
                            td { style: "padding: 8px 12px;", "User ID for manual changes; 'system' for automated transitions" }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
// Toggles tab
// ============================================================

#[component]
fn TogglesTab(can_write: bool) -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 16px;",

            if !can_write {
                div { style: "padding: 10px 14px; background: var(--color-warning-light, #fef3c7); border-left: 3px solid var(--color-warning, #f59e0b); border-radius: 4px; font-size: 0.875rem;",
                    strong { "Read-only view. " }
                    "Modifying degradation toggles requires the "
                    code { "api.ops.write" }
                    " permission (System Administrator role)."
                }
            }

            // Exports toggle
            div { class: "card",
                div { style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;",
                    div {
                        h3 { style: "margin-bottom: 6px;", "Exports" }
                        p { style: "font-size: 0.875rem; color: var(--color-text-secondary); max-width: 480px;",
                            "Controls whether "
                            code { "POST /api/reports/export" }
                            " is available. When disabled, all export requests return 503 Service Unavailable. "
                            "Use this toggle to shed load during peak hours or before a major migration."
                        }
                        p { style: "font-size: 0.75rem; color: var(--color-text-secondary); margin-top: 8px;",
                            "Toggle key: "
                            code { "exports_enabled" }
                            " — fail-open (true if DB unreachable)"
                        }
                    }
                    div { style: "display: flex; flex-direction: column; align-items: center; gap: 8px; min-width: 120px;",
                        TogglePill { enabled: true, can_write: can_write, flag_key: "exports_enabled" }
                    }
                }
            }

            // Analytics toggle
            div { class: "card",
                div { style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; flex-wrap: wrap;",
                    div {
                        h3 { style: "margin-bottom: 6px;", "Heavy Analytics" }
                        p { style: "font-size: 0.875rem; color: var(--color-text-secondary); max-width: 480px;",
                            "Controls whether "
                            code { "GET /api/reports/kpi" }
                            ", "
                            code { "/order-volume" }
                            ", "
                            code { "/revenue" }
                            ", and "
                            code { "/utilization" }
                            " run their aggregate SQL queries. "
                            "When disabled, all report endpoints return 503. Use during high-traffic windows."
                        }
                        p { style: "font-size: 0.75rem; color: var(--color-text-secondary); margin-top: 8px;",
                            "Toggle key: "
                            code { "analytics_enabled" }
                            " — fail-open (true if DB unreachable)"
                        }
                    }
                    div { style: "display: flex; flex-direction: column; align-items: center; gap: 8px; min-width: 120px;",
                        TogglePill { enabled: true, can_write: can_write, flag_key: "analytics_enabled" }
                    }
                }
            }

            // Audit note
            div { class: "card", style: "background: var(--color-bg-secondary, #f8f9fa);",
                h3 { style: "margin-bottom: 8px;", "Audit Trail" }
                p { style: "font-size: 0.875rem;",
                    "Every toggle change is written to both the "
                    code { "audit_log" }
                    " table (resource_type="
                    code { "ops_toggle" }
                    ") and the "
                    code { "ops_events" }
                    " immutable log. The change is also emitted as a structured "
                    code { "tracing::warn!" }
                    " log line with "
                    code { "key" }
                    ", "
                    code { "old_value" }
                    ", "
                    code { "new_value" }
                    ", and "
                    code { "actor_id" }
                    " fields."
                }
            }
        }
    }
}

#[component]
fn TogglePill(enabled: bool, can_write: bool, flag_key: &'static str) -> Element {
    let (bg, label) = if enabled {
        ("background: #dcfce7; color: #166534; border: 1px solid #86efac;", "ENABLED")
    } else {
        ("background: #fee2e2; color: #991b1b; border: 1px solid #fca5a5;", "DISABLED")
    };

    rsx! {
        span { style: "padding: 4px 14px; border-radius: 12px; font-size: 0.8125rem; font-weight: 700; {bg}",
            "{label}"
        }
        if can_write {
            div { style: "display: flex; gap: 6px;",
                button {
                    class: "btn btn-primary",
                    style: "font-size: 0.75rem; padding: 4px 10px;",
                    disabled: enabled,
                    "Enable"
                }
                button {
                    class: "btn btn-secondary",
                    style: "font-size: 0.75rem; padding: 4px 10px;",
                    disabled: !enabled,
                    "Disable"
                }
            }
        }
    }
}

// ============================================================
// Chaos tab
// ============================================================

#[component]
fn ChaosTab() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 16px;",

            // Status
            div { class: "card",
                div { style: "display: flex; align-items: center; gap: 12px; margin-bottom: 16px;",
                    h3 { style: "margin: 0;", "Chaos Drill Status" }
                    span { style: "padding: 3px 10px; border-radius: 12px; font-size: 0.75rem; font-weight: 600; background: #f0fdf4; color: #166534;",
                        "INACTIVE"
                    }
                }
                div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px;",
                    InfoCard { title: "CHAOS_ENABLED", value: "false", note: "Set env var to 'true' to arm" }
                    InfoCard { title: "Drill Window", value: "Sun 02:00–02:15 UTC", note: "Weekly 15-minute window" }
                    InfoCard { title: "In Window Now", value: "No", note: "Check /api/health/chaos live" }
                    InfoCard { title: "Drill Active", value: "No", note: "Armed AND in window" }
                }
            }

            // Guardrails
            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Guardrails" }
                div { style: "display: flex; flex-direction: column; gap: 10px;",
                    GuardrailRow {
                        title: "Environment Gate",
                        description: "Chaos faults only activate when CHAOS_ENABLED=true. Defaults to false. Never enable in production without explicit sign-off."
                    }
                    GuardrailRow {
                        title: "Time-Window Gate",
                        description: "Even when armed, faults only occur on Sunday between 02:00 and 02:14:59 UTC. Outside this window, all service methods behave normally."
                    }
                    GuardrailRow {
                        title: "Bounded Latency",
                        description: "Simulated DB latency is exactly 200 ms (tokio::time::sleep). No unbounded delays or infinite loops."
                    }
                    GuardrailRow {
                        title: "Bounded Timeout Fraction",
                        description: "Timeout injection triggers for at most 5% of requests (subsec_nanos discriminator). 95% of requests are unaffected even during a drill."
                    }
                }
            }

            // Fault types
            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Simulated Faults" }
                table { style: "width: 100%; border-collapse: collapse; font-size: 0.875rem;",
                    thead {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            th { style: "padding: 8px 12px; text-align: left;", "Fault Type" }
                            th { style: "padding: 8px 12px; text-align: left;", "Trigger" }
                            th { style: "padding: 8px 12px; text-align: left;", "Effect" }
                            th { style: "padding: 8px 12px; text-align: left;", "Affected Endpoints" }
                        }
                    }
                    tbody {
                        tr { style: "border-bottom: 1px solid var(--color-border);",
                            td { style: "padding: 8px 12px;", "DB Latency" }
                            td { style: "padding: 8px 12px;", "maybe_inject_latency()" }
                            td { style: "padding: 8px 12px;", "200 ms sleep before response" }
                            td { style: "padding: 8px 12px;", "All report and export calls" }
                        }
                        tr {
                            td { style: "padding: 8px 12px;", "Request Timeout" }
                            td { style: "padding: 8px 12px;", "should_inject_timeout()" }
                            td { style: "padding: 8px 12px;", "5% of calls return true" }
                            td { style: "padding: 8px 12px;", "Caller-level check (logging/alerting)" }
                        }
                    }
                }
            }

            // How to activate
            div { class: "card",
                h3 { style: "margin-bottom: 8px;", "How to Activate a Drill" }
                ol { style: "font-size: 0.875rem; padding-left: 20px; display: flex; flex-direction: column; gap: 6px;",
                    li {
                        "Set the environment variable: "
                        code { "CHAOS_ENABLED=true" }
                        " and restart the backend."
                    }
                    li {
                        "Wait for the Sunday 02:00 UTC window (or adjust the constants for testing)."
                    }
                    li {
                        "Observe latency in "
                        code { "GET /api/health/metrics" }
                        " and check "
                        code { "ops_events" }
                        " for drill_started/drill_stopped log entries."
                    }
                    li {
                        "After the window, set "
                        code { "CHAOS_ENABLED=false" }
                        " (or leave armed for next Sunday)."
                    }
                }
            }
        }
    }
}

#[component]
fn GuardrailRow(title: &'static str, description: &'static str) -> Element {
    rsx! {
        div { style: "display: flex; gap: 12px; align-items: flex-start;",
            span { style: "display: inline-block; width: 8px; height: 8px; border-radius: 50%; background: #16a34a; margin-top: 6px; flex-shrink: 0;" }
            div {
                p { style: "font-weight: 600; font-size: 0.875rem; margin-bottom: 2px;", "{title}" }
                p { style: "font-size: 0.8125rem; color: var(--color-text-secondary);", "{description}" }
            }
        }
    }
}

// ============================================================
// Shared components
// ============================================================

#[component]
fn StatusBadge(label: &'static str, status: &'static str) -> Element {
    let (bg, text) = if status == "ok" {
        ("#dcfce7", "#166534")
    } else {
        ("#fee2e2", "#991b1b")
    };
    rsx! {
        span { style: "display: inline-block; margin-top: 10px; padding: 3px 12px; border-radius: 12px; font-size: 0.8125rem; font-weight: 600; background: {bg}; color: {text};",
            "{label}: {status}"
        }
    }
}

#[component]
fn InfoCard(title: &'static str, value: &'static str, note: &'static str) -> Element {
    rsx! {
        div { style: "background: var(--color-bg-secondary, #f8f9fa); border-radius: 6px; padding: 12px 14px;",
            p { style: "font-size: 0.7rem; color: var(--color-text-secondary); text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 4px;",
                "{title}"
            }
            p { style: "font-size: 1.1rem; font-weight: 700; margin-bottom: 4px;", "{value}" }
            p { style: "font-size: 0.75rem; color: var(--color-text-secondary);", "{note}" }
        }
    }
}

#[component]
fn OpsTabButton(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    let style = if active {
        "padding: 8px 16px; border: none; border-bottom: 2px solid var(--color-primary); background: transparent; font-weight: 600; cursor: pointer; color: var(--color-primary);"
    } else {
        "padding: 8px 16px; border: none; border-bottom: 2px solid transparent; background: transparent; cursor: pointer; color: var(--color-text-secondary);"
    };
    rsx! {
        button { style: style, onclick: move |e| onclick.call(e), "{label}" }
    }
}
