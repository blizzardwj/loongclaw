use serde_json::{Value, json};

use crate::config::{LoongClawConfig, ReasoningEffort};

use super::capability_profile_runtime::ProviderCapabilityProfile;
use super::contracts::{
    CompletionPayloadMode, ProviderCapabilityContract, ProviderRuntimeContract,
    ProviderTransportMode, ReasoningField, TemperatureField, TokenLimitField,
    provider_runtime_contract,
};

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn build_completion_request_body(
    config: &LoongClawConfig,
    messages: &[Value],
    model: &str,
    payload_mode: CompletionPayloadMode,
) -> Value {
    let runtime_contract = provider_runtime_contract(&config.provider);
    let capability_profile =
        ProviderCapabilityProfile::from_provider(&config.provider, runtime_contract);
    let capability = capability_profile.resolve_for_model(model);
    build_completion_request_body_with_capability(
        config,
        messages,
        model,
        payload_mode,
        runtime_contract,
        capability,
    )
}

pub(super) fn build_completion_request_body_with_capability(
    config: &LoongClawConfig,
    messages: &[Value],
    model: &str,
    payload_mode: CompletionPayloadMode,
    runtime_contract: ProviderRuntimeContract,
    capability: ProviderCapabilityContract,
) -> Value {
    match runtime_contract.transport_mode {
        ProviderTransportMode::Responses => {
            build_responses_request_body(config, messages, model, payload_mode, false, &[])
        }
        ProviderTransportMode::OpenAiChatCompletions | ProviderTransportMode::KimiApi => {
            build_chat_completions_request_body(config, messages, model, payload_mode, capability)
        }
    }
}

fn build_chat_completions_request_body(
    config: &LoongClawConfig,
    messages: &[Value],
    model: &str,
    payload_mode: CompletionPayloadMode,
    capability: ProviderCapabilityContract,
) -> Value {
    let mut body = serde_json::Map::new();
    body.insert("model".to_owned(), json!(model));
    body.insert("messages".to_owned(), Value::Array(messages.to_vec()));
    body.insert("stream".to_owned(), Value::Bool(false));
    apply_common_payload_fields(&mut body, config, payload_mode, capability);

    Value::Object(body)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn build_turn_request_body(
    config: &LoongClawConfig,
    messages: &[Value],
    model: &str,
    payload_mode: CompletionPayloadMode,
    include_tool_schema: bool,
    tool_definitions: &[Value],
) -> Value {
    let runtime_contract = provider_runtime_contract(&config.provider);
    let capability_profile =
        ProviderCapabilityProfile::from_provider(&config.provider, runtime_contract);
    let capability = capability_profile.resolve_for_model(model);
    build_turn_request_body_with_capability(
        config,
        messages,
        model,
        payload_mode,
        runtime_contract,
        capability,
        include_tool_schema,
        tool_definitions,
    )
}

pub(super) fn build_turn_request_body_with_capability(
    config: &LoongClawConfig,
    messages: &[Value],
    model: &str,
    payload_mode: CompletionPayloadMode,
    runtime_contract: ProviderRuntimeContract,
    capability: ProviderCapabilityContract,
    include_tool_schema: bool,
    tool_definitions: &[Value],
) -> Value {
    match runtime_contract.transport_mode {
        ProviderTransportMode::Responses => build_responses_request_body(
            config,
            messages,
            model,
            payload_mode,
            include_tool_schema,
            tool_definitions,
        ),
        ProviderTransportMode::OpenAiChatCompletions | ProviderTransportMode::KimiApi => {
            let mut body = build_chat_completions_request_body(
                config,
                messages,
                model,
                payload_mode,
                capability,
            );
            if include_tool_schema
                && !tool_definitions.is_empty()
                && let Some(object) = body.as_object_mut()
            {
                object.insert("tools".to_owned(), Value::Array(tool_definitions.to_vec()));
                object.insert("tool_choice".to_owned(), json!("auto"));
            }
            body
        }
    }
}

fn build_responses_request_body(
    config: &LoongClawConfig,
    messages: &[Value],
    model: &str,
    payload_mode: CompletionPayloadMode,
    include_tool_schema: bool,
    tool_definitions: &[Value],
) -> Value {
    let mut body = serde_json::Map::new();
    body.insert("model".to_owned(), json!(model));
    body.insert("stream".to_owned(), Value::Bool(false));

    let (instructions, input_items) = build_responses_input_items(messages);
    if let Some(instructions) = instructions {
        body.insert("instructions".to_owned(), json!(instructions));
    }
    body.insert("input".to_owned(), Value::Array(input_items));
    apply_common_reasoning_and_temperature_fields(&mut body, config, payload_mode);

    if let Some(limit) = config.provider.max_tokens {
        match payload_mode.token_field {
            TokenLimitField::MaxOutputTokens => {
                body.insert("max_output_tokens".to_owned(), json!(limit));
            }
            TokenLimitField::MaxCompletionTokens => {
                body.insert("max_completion_tokens".to_owned(), json!(limit));
            }
            TokenLimitField::MaxTokens => {
                body.insert("max_tokens".to_owned(), json!(limit));
            }
            TokenLimitField::Omit => {}
        }
    }

    if include_tool_schema && !tool_definitions.is_empty() {
        body.insert("tools".to_owned(), Value::Array(tool_definitions.to_vec()));
        body.insert("tool_choice".to_owned(), json!("auto"));
    }

    Value::Object(body)
}

fn build_responses_input_items(messages: &[Value]) -> (Option<String>, Vec<Value>) {
    let mut instructions = Vec::new();
    let mut input_items = Vec::new();
    let mut seen_non_system_message = false;

    for message in messages {
        if let Some(native_item) = normalize_responses_native_input_item(message) {
            seen_non_system_message = true;
            input_items.push(native_item);
            continue;
        }

        let Some(role) = message.get("role").and_then(Value::as_str) else {
            continue;
        };
        let Some(text) = extract_request_message_text(message.get("content")) else {
            continue;
        };
        if role == "system" && !seen_non_system_message {
            instructions.push(text);
            continue;
        }
        seen_non_system_message = true;
        input_items.push(json!({
            "role": role,
            "content": [{
                "type": "input_text",
                "text": text,
            }],
        }));
    }

    let merged_instructions = if instructions.is_empty() {
        None
    } else {
        Some(instructions.join("\n\n"))
    };

    (merged_instructions, input_items)
}

fn normalize_responses_native_input_item(message: &Value) -> Option<Value> {
    let item_type = message.get("type").and_then(Value::as_str)?;
    match item_type {
        "function_call" | "function_call_output" | "reasoning" => Some(message.clone()),
        _ => None,
    }
}

fn extract_request_message_text(content: Option<&Value>) -> Option<String> {
    let content = content?;
    if let Some(text) = content.as_str() {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        return Some(trimmed.to_owned());
    }

    let parts = content.as_array()?;
    let mut merged = Vec::new();
    for part in parts {
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                merged.push(trimmed.to_owned());
            }
            continue;
        }
        if let Some(text) = part
            .get("text")
            .and_then(|value| value.get("value"))
            .and_then(Value::as_str)
        {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                merged.push(trimmed.to_owned());
            }
        }
    }

    if merged.is_empty() {
        return None;
    }
    Some(merged.join("\n"))
}

fn apply_common_payload_fields(
    body: &mut serde_json::Map<String, Value>,
    config: &LoongClawConfig,
    payload_mode: CompletionPayloadMode,
    capability: ProviderCapabilityContract,
) {
    apply_common_reasoning_and_temperature_fields(body, config, payload_mode);

    if let Some(limit) = config.provider.max_tokens {
        match payload_mode.token_field {
            TokenLimitField::MaxCompletionTokens => {
                body.insert("max_completion_tokens".to_owned(), json!(limit));
            }
            TokenLimitField::MaxTokens => {
                body.insert("max_tokens".to_owned(), json!(limit));
            }
            TokenLimitField::MaxOutputTokens | TokenLimitField::Omit => {}
        }
    }

    if capability.include_reasoning_extra_body()
        && let Some(extra_body) = kimi_extra_body(config.provider.reasoning_effort)
    {
        body.insert("extra_body".to_owned(), extra_body);
    }
}

fn apply_common_reasoning_and_temperature_fields(
    body: &mut serde_json::Map<String, Value>,
    config: &LoongClawConfig,
    payload_mode: CompletionPayloadMode,
) {
    if payload_mode.temperature_field == TemperatureField::Include {
        body.insert("temperature".to_owned(), json!(config.provider.temperature));
    }

    if let Some(reasoning_effort) = config.provider.reasoning_effort {
        match payload_mode.reasoning_field {
            ReasoningField::ReasoningEffort => {
                body.insert(
                    "reasoning_effort".to_owned(),
                    json!(reasoning_effort.as_str()),
                );
            }
            ReasoningField::ReasoningObject => {
                body.insert(
                    "reasoning".to_owned(),
                    json!({
                        "effort": reasoning_effort.as_str()
                    }),
                );
            }
            ReasoningField::Omit => {}
        }
    }
}

fn kimi_extra_body(reasoning_effort: Option<ReasoningEffort>) -> Option<Value> {
    let reasoning_effort = reasoning_effort?;
    let thinking_type = match reasoning_effort {
        ReasoningEffort::None => "disabled",
        ReasoningEffort::Minimal
        | ReasoningEffort::Low
        | ReasoningEffort::Medium
        | ReasoningEffort::High
        | ReasoningEffort::Xhigh => "enabled",
    };
    Some(json!({
        "thinking": {
            "type": thinking_type
        }
    }))
}
