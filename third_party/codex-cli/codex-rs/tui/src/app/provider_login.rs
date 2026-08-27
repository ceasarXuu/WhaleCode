//! Provider credential recovery and catalog refresh for routed TUI selections.

use std::sync::Arc;

use super::App;
use crate::app_event::AppEvent;
use crate::app_server_session::AppServerSession;
use crate::chatwidget::PendingProviderSelection;
use crate::model_catalog::ModelCatalog;
use codex_app_server_protocol::LoginAccountParams;
use codex_app_server_protocol::LoginAccountResponse;
use codex_app_server_protocol::ProviderModelAvailability;
use codex_protocol::ProviderAccessMethod;
use codex_protocol::ProviderRoute;
use codex_protocol::openai_models::ReasoningEffort;

impl App {
    pub(super) async fn select_provider_model(
        &mut self,
        app_server: &mut AppServerSession,
        route: ProviderRoute,
        model: Option<String>,
        effort: Option<ReasoningEffort>,
    ) {
        let model = model.or_else(|| {
            self.model_catalog
                .default_model_for_route(&route)
                .map(|model| model.model.clone())
        });
        if matches!(
            self.model_catalog.provider_availability(&route),
            Some(ProviderModelAvailability::MissingCredentials)
        ) {
            self.begin_provider_login(app_server, route, model, effort)
                .await;
            return;
        }

        let was_running = self.chat_widget.is_task_running();
        let route_label = format!("{} ({:?})", route.model_provider_id, route.access_method);
        let Some(updated) = self
            .sync_active_thread_provider_model_setting(
                app_server,
                route.clone(),
                model.clone(),
                effort.clone(),
            )
            .await
        else {
            return;
        };
        match crate::config_update::write_config_batch(
            app_server.request_handle(),
            crate::config_update::build_provider_model_selection_edits(
                &route,
                model.as_deref(),
                effort.as_ref(),
            ),
        )
        .await
        {
            Ok(_) => tracing::info!(
                provider = %route.model_provider_id,
                access_method = ?route.access_method,
                model = ?model,
                effort = ?effort,
                "persisted provider model selection as new-session default"
            ),
            Err(err) => {
                tracing::error!(error = %err, "failed to persist provider model selection");
                self.chat_widget.add_error_message(format!(
                    "Model changed for this session, but its new-session default could not be saved: {}",
                    crate::config_update::format_config_error(&err)
                ));
            }
        }
        if updated && was_running {
            self.chat_widget.add_info_message(
                format!("Provider {route_label} will apply from the next turn."),
                None,
            );
        }
    }

    async fn begin_provider_login(
        &mut self,
        app_server: &mut AppServerSession,
        route: ProviderRoute,
        model: Option<String>,
        effort: Option<ReasoningEffort>,
    ) {
        if route.access_method == ProviderAccessMethod::ApiKey {
            self.chat_widget
                .show_provider_api_key_prompt(route, model, effort);
            return;
        }
        if route.model_provider_id != "openai" {
            self.chat_widget.add_error_message(format!(
                "Browser login is not supported for provider {}.",
                route.model_provider_id
            ));
            return;
        }

        self.chat_widget
            .set_pending_provider_selection(PendingProviderSelection {
                route,
                model,
                effort,
                login_id: None,
            });
        let response = app_server
            .login_account(LoginAccountParams::Chatgpt {
                codex_streamlined_login: false,
                use_hosted_login_success_page: false,
                app_brand: None,
            })
            .await;
        match response {
            Ok(LoginAccountResponse::Chatgpt { login_id, auth_url }) => {
                if let Some(selection) = self.chat_widget.pending_provider_selection_mut() {
                    selection.login_id = Some(login_id);
                }
                self.open_url_in_browser(auth_url.clone());
                self.chat_widget.add_info_message(
                    format!("Complete OpenAI subscription sign-in in your browser: {auth_url}"),
                    None,
                );
            }
            Ok(other) => {
                self.chat_widget.take_pending_provider_selection();
                self.chat_widget
                    .add_error_message(format!("Unexpected provider login response: {other:?}"));
            }
            Err(err) => {
                self.chat_widget.take_pending_provider_selection();
                self.chat_widget
                    .add_error_message(format!("Failed to start provider login: {err}"));
            }
        }
    }

    pub(super) async fn submit_provider_api_key(
        &mut self,
        app_server: &mut AppServerSession,
        selection: PendingProviderSelection,
        api_key: String,
    ) {
        let api_key = api_key.trim().to_string();
        if api_key.is_empty() {
            self.chat_widget
                .add_error_message("Provider API key cannot be empty.".to_string());
            self.chat_widget.show_provider_api_key_prompt(
                selection.route,
                selection.model,
                selection.effort,
            );
            return;
        }
        let params = match selection.route.model_provider_id.as_str() {
            "deepseek" => LoginAccountParams::DeepseekApiKey { api_key },
            "openai" => LoginAccountParams::ApiKey { api_key },
            provider => {
                self.chat_widget.add_error_message(format!(
                    "API key login is not supported for provider {provider}."
                ));
                return;
            }
        };
        match (
            selection.route.model_provider_id.as_str(),
            app_server.login_account(params).await,
        ) {
            ("openai", Ok(LoginAccountResponse::ApiKey {}))
            | ("deepseek", Ok(LoginAccountResponse::DeepseekApiKey {})) => {
                self.finish_provider_login(app_server, selection).await;
            }
            (_, Ok(other)) => self
                .chat_widget
                .add_error_message(format!("Unexpected provider login response: {other:?}")),
            (_, Err(err)) => self
                .chat_widget
                .add_error_message(format!("Failed to save provider API key: {err}")),
        }
    }

    pub(super) async fn provider_login_completed(
        &mut self,
        app_server: &mut AppServerSession,
        login_id: Option<String>,
        success: bool,
        error: Option<String>,
    ) {
        let Some(selection) = self.chat_widget.take_pending_provider_selection() else {
            return;
        };
        if selection.login_id != login_id {
            self.chat_widget.set_pending_provider_selection(selection);
            return;
        }
        if !success {
            self.chat_widget
                .add_error_message(error.unwrap_or_else(|| "Provider login failed.".to_string()));
            return;
        }
        self.finish_provider_login(app_server, selection).await;
    }

    async fn finish_provider_login(
        &mut self,
        app_server: &mut AppServerSession,
        selection: PendingProviderSelection,
    ) {
        let (models, groups) = match app_server.refresh_model_catalog().await {
            Ok(catalog) => catalog,
            Err(err) => {
                self.chat_widget.add_error_message(format!(
                    "Provider login succeeded, but model refresh failed: {err}"
                ));
                return;
            }
        };
        let catalog = Arc::new(ModelCatalog::with_provider_groups(models, groups));
        self.model_catalog = catalog.clone();
        self.chat_widget.set_model_catalog(catalog);
        if !matches!(
            self.model_catalog.provider_availability(&selection.route),
            Some(ProviderModelAvailability::Available)
        ) {
            self.chat_widget.add_error_message(
                "Credentials were saved, but this provider route is still unavailable.".to_string(),
            );
            return;
        }
        self.app_event_tx.send(AppEvent::SelectProviderModel {
            route: selection.route,
            model: selection.model,
            effort: selection.effort,
        });
    }
}
