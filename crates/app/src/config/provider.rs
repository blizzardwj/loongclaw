use std::{collections::BTreeMap, env, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::shared::{
    ConfigValidationIssue, EnvPointerValidationHint, default_loongclaw_home, expand_path,
    validate_env_pointer_field,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderProfile {
    pub id: &'static str,
    pub display_name: &'static str,
    pub base_url: &'static str,
    pub chat_completions_path: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderWireApi {
    #[default]
    ChatCompletions,
    Responses,
}

impl ProviderWireApi {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ChatCompletions => "chat_completions",
            Self::Responses => "responses",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "chat_completions" => Some(Self::ChatCompletions),
            "responses" => Some(Self::Responses),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderTransportReadinessLevel {
    Ready,
    Review,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderTransportReadiness {
    pub level: ProviderTransportReadinessLevel,
    pub summary: String,
    pub detail: String,
    pub auto_fallback_to_chat_completions: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderTransportFallback {
    pub wire_api: ProviderWireApi,
    pub endpoint: String,
    pub provider: ProviderConfig,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderTransportPolicy {
    pub request_wire_api: ProviderWireApi,
    pub request_endpoint: String,
    pub models_endpoint: String,
    pub readiness: ProviderTransportReadiness,
    pub fallback: Option<ProviderTransportFallback>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

impl ReasoningEffort {
    pub const fn as_str(self) -> &'static str {
        match self {
            ReasoningEffort::None => "none",
            ReasoningEffort::Minimal => "minimal",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
            ReasoningEffort::Xhigh => "xhigh",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    #[serde(alias = "anthropic_compatible")]
    Anthropic,
    #[serde(alias = "kimi_compatible")]
    Kimi,
    #[serde(alias = "kimi_coding_compatible")]
    KimiCoding,
    #[serde(alias = "minimax_compatible")]
    Minimax,
    #[serde(alias = "ollama_compatible")]
    Ollama,
    #[default]
    #[serde(alias = "openai_compatible")]
    Openai,
    #[serde(alias = "openrouter_compatible")]
    Openrouter,
    #[serde(alias = "volcengine_custom", alias = "volcengine_compatible")]
    Volcengine,
    #[serde(alias = "xai_compatible")]
    Xai,
    #[serde(alias = "zai_compatible")]
    Zai,
    #[serde(alias = "zhipu_compatible")]
    Zhipu,
    #[serde(alias = "deepseek_compatible")]
    Deepseek,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProfileStateBackendKind {
    #[default]
    File,
    Sqlite,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProfileHealthModeConfig {
    #[default]
    ProviderDefault,
    Enforce,
    ObserveOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderToolSchemaModeConfig {
    #[default]
    ProviderDefault,
    Disabled,
    EnabledStrict,
    EnabledWithDowngrade,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderReasoningExtraBodyModeConfig {
    #[default]
    ProviderDefault,
    Omit,
    KimiThinking,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
    #[serde(default)]
    pub kind: ProviderKind,
    #[serde(default = "default_provider_model")]
    pub model: String,
    #[serde(default = "default_provider_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub wire_api: ProviderWireApi,
    #[serde(default = "default_openai_chat_path")]
    pub chat_completions_path: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub models_endpoint: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub oauth_access_token: Option<String>,
    #[serde(default)]
    pub oauth_access_token_env: Option<String>,
    #[serde(default)]
    pub preferred_models: Vec<String>,
    #[serde(default)]
    pub reasoning_effort: Option<ReasoningEffort>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default = "default_provider_timeout_ms")]
    pub request_timeout_ms: u64,
    #[serde(default = "default_provider_retry_max_attempts")]
    pub retry_max_attempts: usize,
    #[serde(default = "default_provider_retry_initial_backoff_ms")]
    pub retry_initial_backoff_ms: u64,
    #[serde(default = "default_provider_retry_max_backoff_ms")]
    pub retry_max_backoff_ms: u64,
    #[serde(default = "default_model_catalog_cache_ttl_ms")]
    pub model_catalog_cache_ttl_ms: u64,
    #[serde(default = "default_model_catalog_stale_if_error_ms")]
    pub model_catalog_stale_if_error_ms: u64,
    #[serde(default = "default_model_catalog_cache_max_entries")]
    pub model_catalog_cache_max_entries: usize,
    #[serde(default = "default_model_candidate_cooldown_ms")]
    pub model_candidate_cooldown_ms: u64,
    #[serde(default = "default_model_candidate_cooldown_max_ms")]
    pub model_candidate_cooldown_max_ms: u64,
    #[serde(default = "default_model_candidate_cooldown_max_entries")]
    pub model_candidate_cooldown_max_entries: usize,
    #[serde(default = "default_profile_cooldown_ms")]
    pub profile_cooldown_ms: u64,
    #[serde(default = "default_profile_cooldown_max_ms")]
    pub profile_cooldown_max_ms: u64,
    #[serde(default = "default_profile_auth_reject_disable_ms")]
    pub profile_auth_reject_disable_ms: u64,
    #[serde(default = "default_profile_state_max_entries")]
    pub profile_state_max_entries: usize,
    #[serde(default)]
    pub profile_state_backend: ProviderProfileStateBackendKind,
    #[serde(default)]
    pub profile_state_sqlite_path: Option<String>,
    #[serde(default)]
    pub profile_health_mode: ProviderProfileHealthModeConfig,
    #[serde(default)]
    pub tool_schema_mode: ProviderToolSchemaModeConfig,
    #[serde(default)]
    pub reasoning_extra_body_mode: ProviderReasoningExtraBodyModeConfig,
    #[serde(default)]
    pub tool_schema_disabled_model_hints: Vec<String>,
    #[serde(default)]
    pub tool_schema_strict_model_hints: Vec<String>,
    #[serde(default)]
    pub reasoning_extra_body_kimi_model_hints: Vec<String>,
    #[serde(default)]
    pub reasoning_extra_body_omit_model_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderProfileConfig {
    #[serde(default)]
    pub default_for_kind: bool,
    #[serde(flatten)]
    pub provider: ProviderConfig,
}

impl Default for ProviderProfileConfig {
    fn default() -> Self {
        Self {
            default_for_kind: false,
            provider: ProviderConfig::default(),
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            kind: ProviderKind::Openai,
            model: default_provider_model(),
            base_url: default_provider_base_url(),
            wire_api: ProviderWireApi::ChatCompletions,
            chat_completions_path: default_openai_chat_path(),
            endpoint: None,
            models_endpoint: None,
            api_key: None,
            api_key_env: None,
            oauth_access_token: None,
            oauth_access_token_env: None,
            preferred_models: Vec::new(),
            reasoning_effort: None,
            headers: BTreeMap::new(),
            temperature: default_temperature(),
            max_tokens: None,
            request_timeout_ms: default_provider_timeout_ms(),
            retry_max_attempts: default_provider_retry_max_attempts(),
            retry_initial_backoff_ms: default_provider_retry_initial_backoff_ms(),
            retry_max_backoff_ms: default_provider_retry_max_backoff_ms(),
            model_catalog_cache_ttl_ms: default_model_catalog_cache_ttl_ms(),
            model_catalog_stale_if_error_ms: default_model_catalog_stale_if_error_ms(),
            model_catalog_cache_max_entries: default_model_catalog_cache_max_entries(),
            model_candidate_cooldown_ms: default_model_candidate_cooldown_ms(),
            model_candidate_cooldown_max_ms: default_model_candidate_cooldown_max_ms(),
            model_candidate_cooldown_max_entries: default_model_candidate_cooldown_max_entries(),
            profile_cooldown_ms: default_profile_cooldown_ms(),
            profile_cooldown_max_ms: default_profile_cooldown_max_ms(),
            profile_auth_reject_disable_ms: default_profile_auth_reject_disable_ms(),
            profile_state_max_entries: default_profile_state_max_entries(),
            profile_state_backend: ProviderProfileStateBackendKind::default(),
            profile_state_sqlite_path: None,
            profile_health_mode: ProviderProfileHealthModeConfig::default(),
            tool_schema_mode: ProviderToolSchemaModeConfig::default(),
            reasoning_extra_body_mode: ProviderReasoningExtraBodyModeConfig::default(),
            tool_schema_disabled_model_hints: Vec::new(),
            tool_schema_strict_model_hints: Vec::new(),
            reasoning_extra_body_kimi_model_hints: Vec::new(),
            reasoning_extra_body_omit_model_hints: Vec::new(),
        }
    }
}

impl ProviderConfig {
    pub fn fresh_for_kind(kind: ProviderKind) -> Self {
        let mut provider = Self::default();
        provider.kind = kind;
        provider.model = kind.default_model().unwrap_or("auto").to_owned();
        provider.selection_baseline()
    }

    pub(super) fn validate(&self) -> Vec<ConfigValidationIssue> {
        self.validate_with_field_prefix("provider")
    }

    pub(super) fn validate_with_field_prefix(
        &self,
        field_prefix: &str,
    ) -> Vec<ConfigValidationIssue> {
        let mut issues = Vec::new();
        let api_key_env_field_path = format!("{field_prefix}.api_key_env");
        let api_key_inline_field_path = format!("{field_prefix}.api_key");
        let api_key_example = self
            .kind
            .default_api_key_env()
            .unwrap_or("PROVIDER_API_KEY");
        if let Err(issue) = validate_env_pointer_field(
            api_key_env_field_path.as_str(),
            self.api_key_env.as_deref(),
            EnvPointerValidationHint {
                inline_field_path: api_key_inline_field_path.as_str(),
                example_env_name: api_key_example,
                detect_telegram_token_shape: false,
            },
        ) {
            issues.push(*issue);
        }
        let oauth_env_field_path = format!("{field_prefix}.oauth_access_token_env");
        let oauth_inline_field_path = format!("{field_prefix}.oauth_access_token");
        let oauth_example = self
            .kind
            .default_oauth_access_token_env()
            .unwrap_or("PROVIDER_OAUTH_ACCESS_TOKEN");
        if let Err(issue) = validate_env_pointer_field(
            oauth_env_field_path.as_str(),
            self.oauth_access_token_env.as_deref(),
            EnvPointerValidationHint {
                inline_field_path: oauth_inline_field_path.as_str(),
                example_env_name: oauth_example,
                detect_telegram_token_shape: false,
            },
        ) {
            issues.push(*issue);
        }
        issues
    }

    pub fn endpoint(&self) -> String {
        if let Some(endpoint) = non_empty(self.endpoint.as_deref()) {
            return endpoint.to_owned();
        }

        let profile = self.kind.profile();
        let resolved_base_url =
            self.resolve_base_url(profile.base_url, default_provider_base_url().as_str());
        let resolved_chat_path = self.resolve_chat_path(
            profile.chat_completions_path,
            default_openai_chat_path().as_str(),
            default_provider_base_url().as_str(),
        );
        let resolved_request_path = match self.wire_api {
            ProviderWireApi::ChatCompletions => resolved_chat_path,
            ProviderWireApi::Responses => derive_responses_path(&resolved_chat_path),
        };
        join_base_with_path(
            &resolved_base_url,
            &resolved_request_path,
            default_request_path_for_wire_api(self.wire_api).as_str(),
        )
    }

    pub fn models_endpoint(&self) -> String {
        if let Some(endpoint) = non_empty(self.models_endpoint.as_deref()) {
            return endpoint.to_owned();
        }

        let profile = self.kind.profile();
        let resolved_base_url =
            self.resolve_base_url(profile.base_url, default_provider_base_url().as_str());
        let resolved_chat_path = self.resolve_chat_path(
            profile.chat_completions_path,
            default_openai_chat_path().as_str(),
            default_provider_base_url().as_str(),
        );
        let request_path = match self.wire_api {
            ProviderWireApi::ChatCompletions => resolved_chat_path,
            ProviderWireApi::Responses => derive_responses_path(&resolved_chat_path),
        };
        let models_path = derive_models_path(&request_path);
        join_base_with_path(&resolved_base_url, &models_path, "/v1/models")
    }

    #[cfg(test)]
    pub fn default_api_key_env(&self) -> Option<String> {
        self.kind.default_api_key_env().map(str::to_owned)
    }

    #[cfg(test)]
    pub fn default_oauth_access_token_env(&self) -> Option<String> {
        self.kind
            .default_oauth_access_token_env()
            .map(str::to_owned)
    }

    pub fn authorization_header(&self) -> Option<String> {
        if let Some(token) = self.oauth_access_token() {
            return Some(format!("Bearer {token}"));
        }
        self.api_key().map(|key| format!("Bearer {key}"))
    }

    pub fn transport_policy(&self) -> ProviderTransportPolicy {
        let request_endpoint = self.endpoint();
        let models_endpoint = self.models_endpoint();
        let fallback = self.build_responses_fallback();

        let readiness = match self.wire_api {
            ProviderWireApi::ChatCompletions => ProviderTransportReadiness {
                level: ProviderTransportReadinessLevel::Ready,
                summary: "chat_completions compatibility mode".to_owned(),
                detail: format!(
                    "`{}` uses the broadly compatible chat-completions transport at {}",
                    self.kind.profile().id,
                    request_endpoint
                ),
                auto_fallback_to_chat_completions: false,
            },
            ProviderWireApi::Responses => {
                if self.kind == ProviderKind::KimiCoding {
                    ProviderTransportReadiness {
                        level: ProviderTransportReadinessLevel::Unsupported,
                        summary: "responses unsupported for kimi_coding".to_owned(),
                        detail:
                            "kimi_coding currently supports only chat_completions; switch wire_api to `chat_completions`"
                                .to_owned(),
                        auto_fallback_to_chat_completions: false,
                    }
                } else if self.kind == ProviderKind::Openai
                    && !self.uses_explicit_endpoint_override()
                    && self.base_url_is_profile_default_like()
                    && self.chat_completions_path_is_profile_default_like()
                {
                    ProviderTransportReadiness {
                        level: ProviderTransportReadinessLevel::Ready,
                        summary: "responses native mode".to_owned(),
                        detail: format!(
                            "native OpenAI Responses endpoint {} is configured",
                            request_endpoint
                        ),
                        auto_fallback_to_chat_completions: false,
                    }
                } else if let Some(fallback) = fallback.as_ref() {
                    ProviderTransportReadiness {
                        level: ProviderTransportReadinessLevel::Review,
                        summary: "responses compatibility mode with chat fallback".to_owned(),
                        detail: format!(
                            "Responses endpoint {} is running in compatibility mode; LoongClaw will retry chat_completions automatically via {} if Responses is rejected",
                            request_endpoint, fallback.endpoint
                        ),
                        auto_fallback_to_chat_completions: true,
                    }
                } else {
                    ProviderTransportReadiness {
                        level: ProviderTransportReadinessLevel::Review,
                        summary: "responses custom endpoint needs review".to_owned(),
                        detail: format!(
                            "Responses uses an explicit endpoint override ({}); verify it accepts Responses or switch to chat_completions manually",
                            request_endpoint
                        ),
                        auto_fallback_to_chat_completions: false,
                    }
                }
            }
        };

        ProviderTransportPolicy {
            request_wire_api: self.wire_api,
            request_endpoint,
            models_endpoint,
            readiness,
            fallback,
        }
    }

    pub fn transport_readiness(&self) -> ProviderTransportReadiness {
        self.transport_policy().readiness
    }

    pub fn preview_transport_summary(&self) -> Option<String> {
        match self.wire_api {
            ProviderWireApi::Responses => Some(self.transport_readiness().summary),
            ProviderWireApi::ChatCompletions => None,
        }
    }

    pub fn responses_fallback_provider(&self) -> Option<Self> {
        self.transport_policy()
            .fallback
            .map(|fallback| fallback.provider)
    }

    fn build_responses_fallback(&self) -> Option<ProviderTransportFallback> {
        if self.wire_api != ProviderWireApi::Responses
            || self.kind == ProviderKind::KimiCoding
            || self.uses_explicit_endpoint_override()
        {
            return None;
        }

        let mut fallback = self.clone();
        fallback.wire_api = ProviderWireApi::ChatCompletions;
        fallback.endpoint = None;
        Some(ProviderTransportFallback {
            wire_api: ProviderWireApi::ChatCompletions,
            endpoint: fallback.endpoint(),
            provider: fallback,
        })
    }

    pub fn resolved_model(&self) -> Option<String> {
        let trimmed = self.model.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("auto") {
            return Some(trimmed.to_owned());
        }
        self.kind.default_model().map(str::to_owned)
    }

    pub fn resolved_model_catalog_cache_ttl_ms(&self) -> u64 {
        clamp_non_negative_u64(self.model_catalog_cache_ttl_ms, 300_000)
    }

    pub fn resolved_model_catalog_stale_if_error_ms(&self) -> u64 {
        clamp_non_negative_u64(self.model_catalog_stale_if_error_ms, 600_000)
    }

    pub fn resolved_model_catalog_cache_max_entries(&self) -> usize {
        clamp_usize_at_least_one(self.model_catalog_cache_max_entries, 256)
    }

    pub fn resolved_model_candidate_cooldown_ms(&self) -> u64 {
        clamp_non_negative_u64(self.model_candidate_cooldown_ms, 3_600_000)
    }

    pub fn resolved_model_candidate_cooldown_max_ms(&self) -> u64 {
        let base = self.resolved_model_candidate_cooldown_ms();
        clamp_u64_with_floor(self.model_candidate_cooldown_max_ms, 86_400_000, base)
    }

    pub fn resolved_model_candidate_cooldown_max_entries(&self) -> usize {
        clamp_usize_at_least_one(self.model_candidate_cooldown_max_entries, 512)
    }

    pub fn resolved_profile_cooldown_ms(&self) -> u64 {
        clamp_non_negative_u64(self.profile_cooldown_ms, 3_600_000)
    }

    pub fn resolved_profile_cooldown_max_ms(&self) -> u64 {
        let base = self.resolved_profile_cooldown_ms();
        clamp_u64_with_floor(self.profile_cooldown_max_ms, 86_400_000, base)
    }

    pub fn resolved_profile_auth_reject_disable_ms(&self) -> u64 {
        self.profile_auth_reject_disable_ms
            .clamp(60_000, 604_800_000)
    }

    pub fn resolved_profile_state_max_entries(&self) -> usize {
        clamp_usize_at_least_one(self.profile_state_max_entries, 1024)
    }

    pub fn resolved_profile_state_backend(&self) -> ProviderProfileStateBackendKind {
        self.profile_state_backend
    }

    pub fn resolved_profile_state_sqlite_path(&self) -> Option<PathBuf> {
        normalize_sqlite_path(self.profile_state_sqlite_path.as_deref())
    }

    pub fn resolved_profile_state_sqlite_path_with_default(&self) -> PathBuf {
        self.resolved_profile_state_sqlite_path()
            .unwrap_or_else(|| default_loongclaw_home().join("provider-profile-state.sqlite3"))
    }

    pub fn resolved_profile_health_mode_config(&self) -> ProviderProfileHealthModeConfig {
        self.profile_health_mode
    }

    pub fn resolved_tool_schema_mode_config(&self) -> ProviderToolSchemaModeConfig {
        self.tool_schema_mode
    }

    pub fn resolved_reasoning_extra_body_mode_config(
        &self,
    ) -> ProviderReasoningExtraBodyModeConfig {
        self.reasoning_extra_body_mode
    }

    pub fn resolved_tool_schema_disabled_model_hints(&self) -> Vec<String> {
        normalize_hint_values(&self.tool_schema_disabled_model_hints)
    }

    pub fn resolved_tool_schema_strict_model_hints(&self) -> Vec<String> {
        normalize_hint_values(&self.tool_schema_strict_model_hints)
    }

    pub fn resolved_reasoning_extra_body_kimi_model_hints(&self) -> Vec<String> {
        normalize_hint_values(&self.reasoning_extra_body_kimi_model_hints)
    }

    pub fn resolved_reasoning_extra_body_omit_model_hints(&self) -> Vec<String> {
        normalize_hint_values(&self.reasoning_extra_body_omit_model_hints)
    }

    pub fn model_selection_requires_fetch(&self) -> bool {
        self.resolved_model().is_none()
    }

    pub fn selection_baseline(&self) -> Self {
        let profile = self.kind.profile();
        let mut baseline = Self::default();
        baseline.kind = self.kind;
        baseline.model = self.model.clone();
        baseline.base_url = profile.base_url.to_owned();
        baseline.wire_api = self.wire_api;
        baseline.chat_completions_path = profile.chat_completions_path.to_owned();
        baseline.api_key_env = self.kind.default_api_key_env().map(str::to_owned);
        baseline.oauth_access_token_env = self
            .kind
            .default_oauth_access_token_env()
            .map(str::to_owned);
        baseline
    }

    pub fn has_only_selection_changes(&self) -> bool {
        self == &self.selection_baseline()
    }

    pub fn differs_from_default(&self) -> bool {
        self != &Self::default()
    }

    pub fn base_url_is_profile_default_like(&self) -> bool {
        let profile = self.kind.profile();
        self.base_url.trim().is_empty()
            || is_same_base_url(self.base_url.as_str(), profile.base_url)
    }

    pub fn chat_completions_path_is_profile_default_like(&self) -> bool {
        let profile = self.kind.profile();
        self.chat_completions_path.trim().is_empty()
            || is_same_chat_path(
                self.chat_completions_path.as_str(),
                profile.chat_completions_path,
            )
    }

    pub fn oauth_access_token(&self) -> Option<String> {
        if let Some(raw) = self.oauth_access_token.as_deref() {
            let value = raw.trim();
            if !value.is_empty() {
                return match resolve_inline_secret(value) {
                    InlineSecretResolution::Resolved(secret) => Some(secret),
                    InlineSecretResolution::ExplicitEnvMissing => None,
                    InlineSecretResolution::NotInlineEnvReference => Some(value.to_owned()),
                };
            }
        }

        let mut env_keys = Vec::new();
        push_unique_env_key(&mut env_keys, self.oauth_access_token_env.as_deref());
        if self.should_apply_default_oauth_env_fallback() {
            push_unique_env_key(&mut env_keys, self.kind.default_oauth_access_token_env());
            for alias in self.kind.oauth_access_token_env_aliases() {
                push_unique_env_key(&mut env_keys, Some(alias));
            }
        }

        collect_non_empty_env_values(&env_keys).into_iter().next()
    }

    fn should_apply_default_oauth_env_fallback(&self) -> bool {
        non_empty(self.endpoint.as_deref()).is_none()
            && self.base_url_is_profile_default_like()
            && self.chat_completions_path_is_profile_default_like()
    }

    fn uses_explicit_endpoint_override(&self) -> bool {
        non_empty(self.endpoint.as_deref()).is_some()
    }

    fn resolve_base_url(&self, profile_default: &str, openai_default: &str) -> String {
        let base = self.base_url.trim();
        if base.is_empty() {
            return profile_default.to_owned();
        }
        if self.kind != ProviderKind::Openai
            && is_same_base_url(base, openai_default)
            && (self.chat_completions_path.trim().is_empty()
                || is_same_chat_path(
                    self.chat_completions_path.as_str(),
                    default_openai_chat_path().as_str(),
                ))
        {
            return profile_default.to_owned();
        }
        base.to_owned()
    }

    fn resolve_chat_path(
        &self,
        profile_default: &str,
        openai_default_path: &str,
        openai_default_base: &str,
    ) -> String {
        let path = self.chat_completions_path.trim();
        if path.is_empty() {
            return profile_default.to_owned();
        }
        if self.kind != ProviderKind::Openai
            && is_same_chat_path(path, openai_default_path)
            && (self.base_url.trim().is_empty()
                || is_same_base_url(self.base_url.as_str(), openai_default_base))
        {
            return profile_default.to_owned();
        }
        normalize_api_path(path)
    }

    pub fn api_key(&self) -> Option<String> {
        self.api_key_candidates().into_iter().next()
    }

    pub fn api_key_candidates(&self) -> Vec<String> {
        if let Some(raw) = self.api_key.as_deref() {
            let value = raw.trim();
            if !value.is_empty() {
                return match resolve_inline_secret(value) {
                    InlineSecretResolution::Resolved(secret) => split_secret_candidates(&secret),
                    InlineSecretResolution::ExplicitEnvMissing => Vec::new(),
                    InlineSecretResolution::NotInlineEnvReference => split_secret_candidates(value),
                };
            }
        }

        let mut env_keys = Vec::new();
        push_unique_env_key(&mut env_keys, self.api_key_env.as_deref());
        push_unique_env_key(&mut env_keys, self.kind.default_api_key_env());
        for alias in self.kind.api_key_env_aliases() {
            push_unique_env_key(&mut env_keys, Some(alias));
        }

        collect_non_empty_env_values(&env_keys)
    }

    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn inferred_profile_id(&self) -> String {
        self.kind.profile().id.to_owned()
    }
}

impl ProviderProfileConfig {
    pub fn from_provider(provider: ProviderConfig) -> Self {
        Self {
            default_for_kind: false,
            provider,
        }
    }
}

impl ProviderKind {
    pub const fn all_sorted() -> &'static [ProviderKind] {
        &[
            ProviderKind::Anthropic,
            ProviderKind::Deepseek,
            ProviderKind::Kimi,
            ProviderKind::KimiCoding,
            ProviderKind::Minimax,
            ProviderKind::Ollama,
            ProviderKind::Openai,
            ProviderKind::Openrouter,
            ProviderKind::Volcengine,
            ProviderKind::Xai,
            ProviderKind::Zai,
            ProviderKind::Zhipu,
        ]
    }

    pub const fn as_str(self) -> &'static str {
        self.profile().id
    }

    pub const fn display_name(self) -> &'static str {
        self.profile().display_name
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "anthropic" | "anthropic_compatible" => Some(ProviderKind::Anthropic),
            "deepseek" | "deepseek_compatible" => Some(ProviderKind::Deepseek),
            "kimi" | "kimi_compatible" => Some(ProviderKind::Kimi),
            "kimi_coding" | "kimi_coding_compatible" => Some(ProviderKind::KimiCoding),
            "minimax" | "minimax_compatible" => Some(ProviderKind::Minimax),
            "ollama" | "ollama_compatible" => Some(ProviderKind::Ollama),
            "openai" | "openai_compatible" => Some(ProviderKind::Openai),
            "openrouter" | "openrouter_compatible" => Some(ProviderKind::Openrouter),
            "volcengine" | "volcengine_custom" | "volcengine_compatible" => {
                Some(ProviderKind::Volcengine)
            }
            "xai" | "xai_compatible" => Some(ProviderKind::Xai),
            "zai" | "zai_compatible" => Some(ProviderKind::Zai),
            "zhipu" | "zhipu_compatible" => Some(ProviderKind::Zhipu),
            _ => None,
        }
    }

    pub const fn profile(self) -> ProviderProfile {
        match self {
            ProviderKind::Anthropic => ProviderProfile {
                id: "anthropic",
                display_name: "Anthropic",
                base_url: "https://api.anthropic.com/v1",
                chat_completions_path: "/chat/completions",
            },
            ProviderKind::Deepseek => ProviderProfile {
                id: "deepseek",
                display_name: "DeepSeek",
                base_url: "https://api.deepseek.com",
                chat_completions_path: "/v1/chat/completions",
            },
            ProviderKind::Kimi => ProviderProfile {
                id: "kimi",
                display_name: "Kimi",
                base_url: "https://api.moonshot.cn",
                chat_completions_path: "/v1/chat/completions",
            },
            ProviderKind::KimiCoding => ProviderProfile {
                id: "kimi_coding",
                display_name: "Kimi Coding",
                base_url: "https://api.kimi.com",
                chat_completions_path: "/coding/v1/chat/completions",
            },
            ProviderKind::Minimax => ProviderProfile {
                id: "minimax",
                display_name: "MiniMax",
                base_url: "https://api.minimaxi.com",
                chat_completions_path: "/v1/chat/completions",
            },
            ProviderKind::Ollama => ProviderProfile {
                id: "ollama",
                display_name: "Ollama",
                base_url: "http://127.0.0.1:11434",
                chat_completions_path: "/v1/chat/completions",
            },
            ProviderKind::Openai => ProviderProfile {
                id: "openai",
                display_name: "OpenAI",
                base_url: "https://api.openai.com",
                chat_completions_path: "/v1/chat/completions",
            },
            ProviderKind::Openrouter => ProviderProfile {
                id: "openrouter",
                display_name: "OpenRouter",
                base_url: "https://openrouter.ai",
                chat_completions_path: "/api/v1/chat/completions",
            },
            ProviderKind::Volcengine => ProviderProfile {
                id: "volcengine",
                display_name: "Volcengine",
                base_url: "https://ark.cn-beijing.volces.com",
                chat_completions_path: "/api/v3/chat/completions",
            },
            ProviderKind::Xai => ProviderProfile {
                id: "xai",
                display_name: "xAI",
                base_url: "https://api.x.ai",
                chat_completions_path: "/v1/chat/completions",
            },
            ProviderKind::Zai => ProviderProfile {
                id: "zai",
                display_name: "Z.ai",
                base_url: "https://api.z.ai",
                chat_completions_path: "/api/paas/v4/chat/completions",
            },
            ProviderKind::Zhipu => ProviderProfile {
                id: "zhipu",
                display_name: "Zhipu",
                base_url: "https://open.bigmodel.cn",
                chat_completions_path: "/api/paas/v4/chat/completions",
            },
        }
    }

    pub const fn default_api_key_env(self) -> Option<&'static str> {
        match self {
            ProviderKind::Anthropic => Some("ANTHROPIC_API_KEY"),
            ProviderKind::Deepseek => Some("DEEPSEEK_API_KEY"),
            ProviderKind::Kimi => Some("MOONSHOT_API_KEY"),
            ProviderKind::KimiCoding => Some("KIMI_CODING_API_KEY"),
            ProviderKind::Minimax => Some("MINIMAX_API_KEY"),
            ProviderKind::Ollama => None,
            ProviderKind::Openai => Some("OPENAI_API_KEY"),
            ProviderKind::Openrouter => Some("OPENROUTER_API_KEY"),
            ProviderKind::Volcengine => Some("ARK_API_KEY"),
            ProviderKind::Xai => Some("XAI_API_KEY"),
            ProviderKind::Zai => Some("ZAI_API_KEY"),
            ProviderKind::Zhipu => Some("ZHIPUAI_API_KEY"),
        }
    }

    pub const fn api_key_env_aliases(self) -> &'static [&'static str] {
        match self {
            ProviderKind::Zhipu => &["ZHIPU_API_KEY"],
            _ => &[],
        }
    }

    pub const fn default_model(self) -> Option<&'static str> {
        match self {
            ProviderKind::KimiCoding => Some("kimi-for-coding"),
            _ => None,
        }
    }

    pub const fn default_user_agent(self) -> Option<&'static str> {
        match self {
            ProviderKind::KimiCoding => Some("KimiCLI/LoongClaw"),
            _ => None,
        }
    }

    pub const fn default_oauth_access_token_env(self) -> Option<&'static str> {
        match self {
            ProviderKind::Openai => Some("OPENAI_CODEX_OAUTH_TOKEN"),
            ProviderKind::Volcengine => Some("VOLCENGINE_CODING_PLAN_OAUTH_TOKEN"),
            _ => None,
        }
    }

    pub const fn oauth_access_token_env_aliases(self) -> &'static [&'static str] {
        match self {
            ProviderKind::Openai => &["OPENAI_OAUTH_ACCESS_TOKEN"],
            ProviderKind::Volcengine => &["ARK_OAUTH_ACCESS_TOKEN"],
            _ => &[],
        }
    }
}

fn default_provider_model() -> String {
    "auto".to_owned()
}

fn default_provider_base_url() -> String {
    "https://api.openai.com".to_owned()
}

fn default_openai_chat_path() -> String {
    "/v1/chat/completions".to_owned()
}

fn default_openai_responses_path() -> String {
    "/v1/responses".to_owned()
}

fn default_request_path_for_wire_api(wire_api: ProviderWireApi) -> String {
    match wire_api {
        ProviderWireApi::ChatCompletions => default_openai_chat_path(),
        ProviderWireApi::Responses => default_openai_responses_path(),
    }
}

const fn default_temperature() -> f64 {
    0.2
}

const fn default_provider_timeout_ms() -> u64 {
    30_000
}

const fn default_provider_retry_max_attempts() -> usize {
    3
}

const fn default_provider_retry_initial_backoff_ms() -> u64 {
    300
}

const fn default_provider_retry_max_backoff_ms() -> u64 {
    3_000
}

const fn default_model_catalog_cache_ttl_ms() -> u64 {
    30_000
}

const fn default_model_catalog_stale_if_error_ms() -> u64 {
    120_000
}

const fn default_model_catalog_cache_max_entries() -> usize {
    32
}

const fn default_model_candidate_cooldown_ms() -> u64 {
    300_000
}

const fn default_model_candidate_cooldown_max_ms() -> u64 {
    3_600_000
}

const fn default_model_candidate_cooldown_max_entries() -> usize {
    64
}

const fn default_profile_cooldown_ms() -> u64 {
    60_000
}

const fn default_profile_cooldown_max_ms() -> u64 {
    3_600_000
}

const fn default_profile_auth_reject_disable_ms() -> u64 {
    21_600_000
}

const fn default_profile_state_max_entries() -> usize {
    256
}

fn collect_non_empty_env_values(keys: &[String]) -> Vec<String> {
    let mut values = Vec::new();
    for key in keys {
        if let Ok(value) = env::var(key) {
            for candidate in split_secret_candidates(&value) {
                push_unique_value(&mut values, &candidate);
            }
        }
    }
    values
}

fn push_unique_env_key(keys: &mut Vec<String>, maybe_key: Option<&str>) {
    let Some(raw) = maybe_key else {
        return;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return;
    }
    if keys.iter().any(|existing| existing == trimmed) {
        return;
    }
    keys.push(trimmed.to_owned());
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    let raw = value?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed)
}

fn clamp_non_negative_u64(value: u64, max: u64) -> u64 {
    if value == 0 { 0 } else { value.min(max) }
}

fn clamp_u64_with_floor(value: u64, max: u64, floor: u64) -> u64 {
    value.clamp(floor, max)
}

fn clamp_usize_at_least_one(value: usize, max: usize) -> usize {
    value.clamp(1, max)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InlineSecretResolution {
    Resolved(String),
    ExplicitEnvMissing,
    NotInlineEnvReference,
}

fn resolve_inline_secret(raw: &str) -> InlineSecretResolution {
    let Some(env_key) = parse_explicit_env_reference(raw) else {
        return InlineSecretResolution::NotInlineEnvReference;
    };
    match env::var(env_key) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                InlineSecretResolution::ExplicitEnvMissing
            } else {
                InlineSecretResolution::Resolved(trimmed.to_owned())
            }
        }
        Err(_) => InlineSecretResolution::ExplicitEnvMissing,
    }
}

fn parse_explicit_env_reference(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if let Some(inner) = trimmed
        .strip_prefix("${")
        .and_then(|value| value.strip_suffix('}'))
    {
        return non_empty(Some(inner.trim()));
    }
    if let Some(inner) = trimmed.strip_prefix('$') {
        return non_empty(Some(inner.trim()));
    }
    if let Some(inner) = trimmed.strip_prefix("env:") {
        return non_empty(Some(inner.trim()));
    }
    if let Some(inner) = trimmed
        .strip_prefix('%')
        .and_then(|value| value.strip_suffix('%'))
    {
        return non_empty(Some(inner.trim()));
    }
    None
}

fn split_secret_candidates(raw: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in raw.split([',', ';', '\n', '\r']) {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        push_unique_value(&mut values, trimmed);
    }
    values
}

fn push_unique_value(values: &mut Vec<String>, raw: &str) {
    if values.iter().any(|existing| existing == raw) {
        return;
    }
    values.push(raw.to_owned());
}

fn normalize_hint_values(values: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for raw in values {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lowercased = trimmed.to_ascii_lowercase();
        if normalized.iter().any(|existing| existing == &lowercased) {
            continue;
        }
        normalized.push(lowercased);
    }
    normalized
}

fn normalize_sqlite_path(raw: Option<&str>) -> Option<PathBuf> {
    let trimmed = non_empty(raw)?;
    if trimmed.eq_ignore_ascii_case("memory") || trimmed == ":memory:" {
        return Some(PathBuf::from(":memory:"));
    }
    Some(expand_path(trimmed))
}

fn normalize_api_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('/') {
        return trimmed.to_owned();
    }
    format!("/{trimmed}")
}

fn is_same_base_url(left: &str, right: &str) -> bool {
    left.trim().trim_end_matches('/') == right.trim().trim_end_matches('/')
}

fn is_same_chat_path(left: &str, right: &str) -> bool {
    normalize_api_path(left) == normalize_api_path(right)
}

fn join_base_with_path(base_url: &str, path: &str, fallback_path: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let path = normalize_api_path(path);
    if path.is_empty() {
        return format!("{base}{}", normalize_api_path(fallback_path));
    }
    format!("{base}{path}")
}

fn derive_models_path(chat_path: &str) -> String {
    let normalized = normalize_api_path(chat_path);
    if normalized.is_empty() {
        return "/v1/models".to_owned();
    }

    if let Some(prefix) = normalized.strip_suffix("/chat/completions") {
        let prefix = if prefix.is_empty() { "" } else { prefix };
        return format!("{prefix}/models");
    }
    if let Some(prefix) = normalized.strip_suffix("/completions") {
        let prefix = if prefix.is_empty() { "" } else { prefix };
        return format!("{prefix}/models");
    }
    if let Some(prefix) = normalized.strip_suffix("/responses") {
        let prefix = if prefix.is_empty() { "" } else { prefix };
        return format!("{prefix}/models");
    }

    "/v1/models".to_owned()
}

fn derive_responses_path(chat_path: &str) -> String {
    let normalized = normalize_api_path(chat_path);
    if normalized.is_empty() {
        return default_openai_responses_path();
    }

    if let Some(prefix) = normalized.strip_suffix("/chat/completions") {
        let prefix = if prefix.is_empty() { "" } else { prefix };
        return format!("{prefix}/responses");
    }
    if let Some(prefix) = normalized.strip_suffix("/completions") {
        let prefix = if prefix.is_empty() { "" } else { prefix };
        return format!("{prefix}/responses");
    }
    if normalized.ends_with("/responses") {
        return normalized;
    }

    default_openai_responses_path()
}
