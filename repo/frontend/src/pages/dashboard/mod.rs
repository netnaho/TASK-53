use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        div {
            div { class: "page-header",
                h1 { "Dashboard" }
                p { "Overview of service delivery, billing status, and quality metrics across your organization." }
            }
            div { style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 16px; margin-bottom: 24px;",
                StatCard { title: "Active Plans", value: "--", context: "Client service plans currently in progress" }
                StatCard { title: "Pending Deliveries", value: "--", context: "Service entries awaiting verification" }
                StatCard { title: "Open Invoices", value: "--", context: "Invoices issued but not yet paid" }
                StatCard { title: "Avg Quality Score", value: "--", context: "Mean score from quality reviews this period" }
            }
            div { class: "card",
                h3 { style: "margin-bottom: 12px;", "Recent Activity" }
                p { style: "color: var(--color-text-secondary);",
                    "Activity feed will display recent delivery entries, billing events, and quality reviews."
                }
            }
        }
    }
}

#[component]
fn StatCard(title: &'static str, value: &'static str, context: &'static str) -> Element {
    rsx! {
        div { class: "card",
            p { style: "font-size: 12px; color: var(--color-text-secondary); text-transform: uppercase; letter-spacing: 0.5px;",
                "{title}"
            }
            p { style: "font-size: 32px; font-weight: 700; margin: 8px 0;", "{value}" }
            p { style: "font-size: 12px; color: var(--color-text-secondary);", "{context}" }
        }
    }
}
