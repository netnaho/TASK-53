use dioxus::prelude::*;
use crate::models::{LoginRequest, LoginResponse};
use crate::services::ApiClient;
use crate::state::AuthState;
use crate::router::Route;

#[component]
pub fn Login() -> Element {
    let mut auth: Signal<AuthState> = use_context();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut error_msg = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let nav = use_navigator();

    // If already authenticated, redirect to dashboard
    if auth.read().is_authenticated() {
        nav.replace(Route::Dashboard {});
    }

    let on_login = move |_: MouseEvent| {
        let u = username.read().clone();
        let p = password.read().clone();

        if u.is_empty() || p.is_empty() {
            error_msg.set(Some("Username and password are required".to_string()));
            return;
        }

        let nav = nav.clone();
        spawn(async move {
            loading.set(true);
            error_msg.set(None);

            let req = LoginRequest { username: u, password: p };
            match ApiClient::post::<LoginResponse, _>("/auth/login", &req, None).await {
                Ok(resp) => {
                    auth.set(AuthState {
                        token: Some(resp.token),
                        user: Some(resp.user),
                    });
                    nav.replace(Route::Dashboard {});
                }
                Err(e) => {
                    let msg = if e.contains("401") {
                        "Invalid username or password".to_string()
                    } else {
                        format!("Login failed: {}", e)
                    };
                    error_msg.set(Some(msg));
                }
            }

            loading.set(false);
        });
    };

    rsx! {
        div { class: "login-page", style: "background: var(--color-bg); min-height: 100vh;",
            div { class: "card", style: "max-width: 420px; margin: 100px auto; padding: 36px;",
                h1 { style: "font-size: 24px; margin-bottom: 4px; color: var(--color-primary);", "CareOps Portal" }
                p { style: "color: var(--color-text-secondary); margin-bottom: 28px; font-size: 14px;",
                    "Sign in to manage service delivery, billing, and quality."
                }

                if let Some(err) = error_msg.read().as_ref() {
                    div {
                        style: "background: #fef2f2; border: 1px solid #fecaca; color: #dc2626; padding: 10px 14px; border-radius: var(--radius); margin-bottom: 20px; font-size: 13px;",
                        "{err}"
                    }
                }

                div {
                    div { style: "margin-bottom: 18px;",
                        label { style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Username" }
                        input {
                            r#type: "text",
                            value: "{username}",
                            oninput: move |evt| username.set(evt.value()),
                            placeholder: "Enter username",
                            disabled: *loading.read(),
                            style: "width: 100%; padding: 10px 12px; border: 1px solid var(--color-border); border-radius: var(--radius); font-size: 14px;"
                        }
                    }
                    div { style: "margin-bottom: 24px;",
                        label { style: "display: block; font-size: 13px; font-weight: 500; margin-bottom: 4px;", "Password" }
                        input {
                            r#type: "password",
                            value: "{password}",
                            oninput: move |evt| password.set(evt.value()),
                            placeholder: "Enter password",
                            disabled: *loading.read(),
                            style: "width: 100%; padding: 10px 12px; border: 1px solid var(--color-border); border-radius: var(--radius); font-size: 14px;"
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        r#type: "button",
                        onclick: on_login,
                        disabled: *loading.read(),
                        style: "width: 100%; padding: 12px; font-size: 15px;",
                        if *loading.read() { "Signing in..." } else { "Sign In" }
                    }
                }

                div { style: "margin-top: 24px; padding-top: 16px; border-top: 1px solid var(--color-border); font-size: 12px; color: var(--color-text-secondary);",
                    p { style: "font-weight: 500; margin-bottom: 6px;", "Demo Accounts" }
                    p { "admin / Admin123! (System Administrator)" }
                    p { "ops_manager / OpsManager123!" }
                    p { "billing_staff / Billing123!" }
                    p { "coach / Coach123!" }
                    p { "qa_reviewer / QAReview123!" }
                    p { "auditor / Auditor123!" }
                }
            }
        }
    }
}
