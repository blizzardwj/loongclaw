use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value;

use crate::config::{LoongClawConfig, ProviderConfig};
use crate::tools;

use super::capability_profile_runtime::ProviderCapabilityProfile;
use super::contracts::{
    ProviderApiError, provider_runtime_contract, should_disable_tool_schema_for_error,
};
use super::failover::ModelRequestError;
use super::policy;
use super::request_executor::{ModelRequestRuntime, execute_model_request};
use super::request_payload_runtime::{
    build_completion_request_body_with_capability, build_turn_request_body_with_capability,
};
use super::shape;

#[allow(clippy::too_many_arguments)]
pub(super) async fn request_completion_with_model(
    config: &LoongClawConfig,
    messages: &[Value],
    model: String,
    auto_model_mode: bool,
    authorization_header: Option<String>,
    headers: &reqwest::header::HeaderMap,
    request_policy: &policy::ProviderRequestPolicy,
    client: &reqwest::Client,
) -> Result<String, ModelRequestError> {
    request_completion_with_provider(
        config,
        &config.provider,
        messages,
        model.as_str(),
        auto_model_mode,
        authorization_header.as_deref(),
        headers,
        request_policy,
        client,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn request_turn_with_model(
    config: &LoongClawConfig,
    messages: &[Value],
    model: String,
    auto_model_mode: bool,
    authorization_header: Option<String>,
    headers: &reqwest::header::HeaderMap,
    request_policy: &policy::ProviderRequestPolicy,
    client: &reqwest::Client,
) -> Result<crate::conversation::turn_engine::ProviderTurn, ModelRequestError> {
    request_turn_with_provider(
        config,
        &config.provider,
        messages,
        model.as_str(),
        auto_model_mode,
        authorization_header.as_deref(),
        headers,
        request_policy,
        client,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn request_completion_with_provider(
    base_config: &LoongClawConfig,
    request_provider: &ProviderConfig,
    messages: &[Value],
    model: &str,
    auto_model_mode: bool,
    authorization_header: Option<&str>,
    headers: &reqwest::header::HeaderMap,
    request_policy: &policy::ProviderRequestPolicy,
    client: &reqwest::Client,
) -> Result<String, ModelRequestError> {
    let mut current_provider = request_provider.clone();
    loop {
        let runtime_contract = provider_runtime_contract(&current_provider);
        let capability_profile =
            ProviderCapabilityProfile::from_provider(&current_provider, runtime_contract);
        let capability = capability_profile.resolve_for_model(model);
        let endpoint = current_provider.endpoint();
        let mut request_config = base_config.clone();
        request_config.provider = current_provider.clone();
        let runtime = ModelRequestRuntime {
            provider: &current_provider,
            model,
            runtime_contract,
            capability,
            auto_model_mode,
            authorization_header,
            endpoint: endpoint.as_str(),
            headers,
            request_policy,
            client,
        };

        match execute_model_request(
            runtime,
            |payload_mode| {
                build_completion_request_body_with_capability(
                    &request_config,
                    messages,
                    model,
                    payload_mode,
                    runtime_contract,
                    capability,
                )
            },
            shape::extract_message_content,
            "choices[0].message.content",
            |_| false,
        )
        .await
        {
            Err(error)
                if should_retry_with_chat_completions_fallback(&current_provider, &error) =>
            {
                if let Some(fallback_provider) = current_provider.responses_fallback_provider() {
                    current_provider = fallback_provider;
                    continue;
                }
                return Err(error);
            }
            result => return result,
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn request_turn_with_provider(
    base_config: &LoongClawConfig,
    request_provider: &ProviderConfig,
    messages: &[Value],
    model: &str,
    auto_model_mode: bool,
    authorization_header: Option<&str>,
    headers: &reqwest::header::HeaderMap,
    request_policy: &policy::ProviderRequestPolicy,
    client: &reqwest::Client,
) -> Result<crate::conversation::turn_engine::ProviderTurn, ModelRequestError> {
    let tool_definitions = tools::provider_tool_definitions();
    let mut current_provider = request_provider.clone();
    loop {
        let runtime_contract = provider_runtime_contract(&current_provider);
        let capability_profile =
            ProviderCapabilityProfile::from_provider(&current_provider, runtime_contract);
        let capability = capability_profile.resolve_for_model(model);
        let include_tool_schema =
            AtomicBool::new(capability.turn_tool_schema_enabled() && !tool_definitions.is_empty());
        let endpoint = current_provider.endpoint();
        let mut request_config = base_config.clone();
        request_config.provider = current_provider.clone();
        let runtime = ModelRequestRuntime {
            provider: &current_provider,
            model,
            runtime_contract,
            capability,
            auto_model_mode,
            authorization_header,
            endpoint: endpoint.as_str(),
            headers,
            request_policy,
            client,
        };

        match execute_model_request(
            runtime,
            |payload_mode| {
                build_turn_request_body_with_capability(
                    &request_config,
                    messages,
                    model,
                    payload_mode,
                    runtime_contract,
                    capability,
                    include_tool_schema.load(Ordering::Relaxed),
                    &tool_definitions,
                )
            },
            shape::extract_provider_turn,
            "choices[0].message",
            |api_error| {
                if include_tool_schema.load(Ordering::Relaxed)
                    && capability.tool_schema_downgrade_on_unsupported()
                    && should_disable_tool_schema_for_error(api_error, runtime_contract)
                {
                    include_tool_schema.store(false, Ordering::Relaxed);
                    return true;
                }
                false
            },
        )
        .await
        {
            Err(error)
                if should_retry_with_chat_completions_fallback(&current_provider, &error) =>
            {
                if let Some(fallback_provider) = current_provider.responses_fallback_provider() {
                    current_provider = fallback_provider;
                    continue;
                }
                return Err(error);
            }
            result => return result,
        }
    }
}

fn should_retry_with_chat_completions_fallback(
    provider: &ProviderConfig,
    error: &ModelRequestError,
) -> bool {
    let Some(status_code) = error.snapshot.status_code else {
        return false;
    };
    let Some(api_error) = error.api_error.as_ref() else {
        return false;
    };
    should_fallback_responses_to_chat_completions(provider, status_code, api_error)
}

fn should_fallback_responses_to_chat_completions(
    provider: &ProviderConfig,
    status_code: u16,
    error: &ProviderApiError,
) -> bool {
    if provider.responses_fallback_provider().is_none()
        || !matches!(status_code, 400 | 404 | 405 | 415 | 422)
    {
        return false;
    }

    let message = error.message.as_deref().unwrap_or_default();
    if message.is_empty()
        || message.contains("unauthorized")
        || message.contains("forbidden")
        || message.contains("invalid api key")
        || message.contains("rate limit")
        || message.contains("insufficient quota")
    {
        return false;
    }

    let mentions_chat_endpoint =
        message.contains("/v1/chat/completions") || message.contains("chat/completions");
    let rejects_responses_input = matches!(error.param.as_deref(), Some("input" | "instructions"))
        && (message.contains("unknown parameter")
            || message.contains("unsupported parameter")
            || message.contains("expects")
            || message.contains("not supported"));
    let requires_messages = error.param.as_deref() == Some("messages")
        && (message.contains("required")
            || message.contains("missing")
            || message.contains("expects"));
    let textual_messages_hint = message.contains("expects `messages`")
        || message.contains("expects messages")
        || message.contains("use `messages`")
        || message.contains("use 'messages'")
        || message.contains("missing required parameter: `messages`")
        || message.contains("requires `messages`")
        || message.contains("requires messages")
        || message.contains("expected `messages`")
        || message.contains("expected messages")
        || message.contains("unknown parameter `input`")
        || message.contains("unknown parameter: `input`")
        || message.contains("unsupported parameter `input`")
        || message.contains("unsupported parameter: `input`")
        || message.contains("unknown parameter `instructions`")
        || message.contains("unknown parameter: `instructions`")
        || message.contains("unsupported parameter `instructions`")
        || message.contains("unsupported parameter: `instructions`");

    mentions_chat_endpoint || rejects_responses_input || requires_messages || textual_messages_hint
}
