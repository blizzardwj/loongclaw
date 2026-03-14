use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::OnceLock;

#[cfg(feature = "feishu-integration")]
use crate::config::{FeishuChannelConfig, FeishuIntegrationConfig, LoongClawConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSkillsRuntimePolicy {
    pub enabled: bool,
    pub require_download_approval: bool,
    pub allowed_domains: BTreeSet<String>,
    pub blocked_domains: BTreeSet<String>,
    pub install_root: Option<PathBuf>,
    pub auto_expose_installed: bool,
}

impl Default for ExternalSkillsRuntimePolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            require_download_approval: true,
            allowed_domains: BTreeSet::new(),
            blocked_domains: BTreeSet::new(),
            install_root: None,
            auto_expose_installed: true,
        }
    }
}

#[cfg(feature = "feishu-integration")]
#[derive(Debug, Clone)]
pub struct FeishuToolRuntimeConfig {
    pub channel: FeishuChannelConfig,
    pub integration: FeishuIntegrationConfig,
}

#[cfg(feature = "feishu-integration")]
impl FeishuToolRuntimeConfig {
    pub fn from_loongclaw_config(config: &LoongClawConfig) -> Option<Self> {
        has_enabled_feishu_runtime_credentials(&config.feishu).then(|| Self {
            channel: config.feishu.clone(),
            integration: config.feishu_integration.clone(),
        })
    }

    fn from_env() -> Option<Self> {
        has_feishu_runtime_credentials(&FeishuChannelConfig::default()).then(|| Self {
            channel: FeishuChannelConfig {
                enabled: true,
                ..FeishuChannelConfig::default()
            },
            integration: FeishuIntegrationConfig::default(),
        })
    }
}

/// Typed runtime configuration for tool executors.
///
/// Replaces per-call `std::env::var` lookups with a single read from a
/// process-wide singleton that is populated once at startup.
#[derive(Debug, Clone, Default)]
pub struct ToolRuntimeConfig {
    pub shell_allowlist: BTreeSet<String>,
    pub file_root: Option<PathBuf>,
    pub external_skills: ExternalSkillsRuntimePolicy,
    #[cfg(feature = "feishu-integration")]
    pub feishu: Option<FeishuToolRuntimeConfig>,
}

impl ToolRuntimeConfig {
    /// Build a config by reading the legacy environment variables.
    ///
    /// Keeps full backward compatibility for callers that still rely on
    /// `LOONGCLAW_SHELL_ALLOWLIST` / `LOONGCLAW_FILE_ROOT`.
    pub fn from_env() -> Self {
        let shell_allowlist = std::env::var("LOONGCLAW_SHELL_ALLOWLIST")
            .ok()
            .unwrap_or_else(|| "echo,pwd".to_owned())
            .split([',', ';', ' '])
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_ascii_lowercase)
            .collect();

        let file_root = std::env::var("LOONGCLAW_FILE_ROOT").ok().map(PathBuf::from);
        let enabled = parse_env_bool("LOONGCLAW_EXTERNAL_SKILLS_ENABLED").unwrap_or(false);
        let require_download_approval =
            parse_env_bool("LOONGCLAW_EXTERNAL_SKILLS_REQUIRE_DOWNLOAD_APPROVAL").unwrap_or(true);
        let allowed_domains = parse_env_domain_list("LOONGCLAW_EXTERNAL_SKILLS_ALLOWED_DOMAINS");
        let blocked_domains = parse_env_domain_list("LOONGCLAW_EXTERNAL_SKILLS_BLOCKED_DOMAINS");
        let install_root = std::env::var("LOONGCLAW_EXTERNAL_SKILLS_INSTALL_ROOT")
            .ok()
            .map(PathBuf::from);
        let auto_expose_installed =
            parse_env_bool("LOONGCLAW_EXTERNAL_SKILLS_AUTO_EXPOSE_INSTALLED").unwrap_or(true);

        Self {
            shell_allowlist,
            file_root,
            external_skills: ExternalSkillsRuntimePolicy {
                enabled,
                require_download_approval,
                allowed_domains,
                blocked_domains,
                install_root,
                auto_expose_installed,
            },
            #[cfg(feature = "feishu-integration")]
            feishu: FeishuToolRuntimeConfig::from_env(),
        }
    }
}

fn parse_env_bool(key: &str) -> Option<bool> {
    std::env::var(key).ok().and_then(|raw| {
        let value = raw.trim().to_ascii_lowercase();
        match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        }
    })
}

fn parse_env_domain_list(key: &str) -> BTreeSet<String> {
    std::env::var(key)
        .ok()
        .unwrap_or_default()
        .split([',', ';', ' '])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(feature = "feishu-integration")]
fn has_enabled_feishu_runtime_credentials(config: &FeishuChannelConfig) -> bool {
    if !config.enabled {
        return false;
    }

    has_secret_binding(config.app_id.as_deref(), config.app_id_env.as_deref())
        && has_secret_binding(
            config.app_secret.as_deref(),
            config.app_secret_env.as_deref(),
        )
        || config
            .accounts
            .values()
            .any(account_has_enabled_feishu_runtime_credentials)
}

#[cfg(feature = "feishu-integration")]
fn has_feishu_runtime_credentials(config: &FeishuChannelConfig) -> bool {
    has_secret_binding(config.app_id.as_deref(), config.app_id_env.as_deref())
        && has_secret_binding(
            config.app_secret.as_deref(),
            config.app_secret_env.as_deref(),
        )
        || config
            .accounts
            .values()
            .any(account_has_feishu_runtime_credentials)
}

#[cfg(feature = "feishu-integration")]
fn account_has_enabled_feishu_runtime_credentials(
    account: &crate::config::FeishuAccountConfig,
) -> bool {
    account.enabled.unwrap_or(true) && account_has_feishu_runtime_credentials(account)
}

#[cfg(feature = "feishu-integration")]
fn account_has_feishu_runtime_credentials(account: &crate::config::FeishuAccountConfig) -> bool {
    has_secret_binding(account.app_id.as_deref(), account.app_id_env.as_deref())
        && has_secret_binding(
            account.app_secret.as_deref(),
            account.app_secret_env.as_deref(),
        )
}

#[cfg(feature = "feishu-integration")]
fn has_secret_binding(inline: Option<&str>, env_name: Option<&str>) -> bool {
    inline
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
        || env_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .and_then(|name| std::env::var(name).ok())
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}

static TOOL_RUNTIME_CONFIG: OnceLock<ToolRuntimeConfig> = OnceLock::new();

/// Initialise the process-wide tool runtime config.
///
/// Returns `Ok(())` on the first call.  Subsequent calls return
/// `Err` because the `OnceLock` rejects duplicate initialisation.
pub fn init_tool_runtime_config(config: ToolRuntimeConfig) -> Result<(), String> {
    TOOL_RUNTIME_CONFIG.set(config).map_err(|_err| {
        "tool runtime config already initialised (duplicate init_tool_runtime_config call)"
            .to_owned()
    })
}

/// Return the process-wide tool runtime config.
///
/// If `init_tool_runtime_config` was never called the config is lazily
/// populated from environment variables (backward-compat path).
pub fn get_tool_runtime_config() -> &'static ToolRuntimeConfig {
    TOOL_RUNTIME_CONFIG.get_or_init(ToolRuntimeConfig::from_env)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "feishu-integration")]
    use std::collections::BTreeMap;

    #[test]
    fn tool_runtime_config_from_env_defaults() {
        let config = ToolRuntimeConfig::default();
        assert!(config.shell_allowlist.is_empty());
        assert!(config.file_root.is_none());
        assert!(!config.external_skills.enabled);
        assert!(config.external_skills.require_download_approval);
        assert!(config.external_skills.allowed_domains.is_empty());
        assert!(config.external_skills.blocked_domains.is_empty());
        assert!(config.external_skills.install_root.is_none());
        assert!(config.external_skills.auto_expose_installed);
    }

    #[test]
    fn shell_allowlist_uses_injected_config_not_env() {
        // Build a ToolRuntimeConfig with an explicit allowlist that differs
        // from any env var that might be set.
        let config = ToolRuntimeConfig {
            shell_allowlist: BTreeSet::from(["git".to_owned(), "cargo".to_owned()]),
            file_root: Some(PathBuf::from("/tmp/test-root")),
            external_skills: ExternalSkillsRuntimePolicy {
                enabled: true,
                require_download_approval: false,
                allowed_domains: BTreeSet::from(["skills.sh".to_owned()]),
                blocked_domains: BTreeSet::new(),
                install_root: Some(PathBuf::from("/tmp/test-root/skills")),
                auto_expose_installed: false,
            },
            ..ToolRuntimeConfig::default()
        };
        assert!(config.shell_allowlist.contains("git"));
        assert!(config.shell_allowlist.contains("cargo"));
        assert!(!config.shell_allowlist.contains("echo"));
        assert_eq!(config.file_root, Some(PathBuf::from("/tmp/test-root")));
        assert!(config.external_skills.enabled);
        assert!(!config.external_skills.require_download_approval);
        assert!(config.external_skills.allowed_domains.contains("skills.sh"));
        assert_eq!(
            config.external_skills.install_root,
            Some(PathBuf::from("/tmp/test-root/skills"))
        );
        assert!(!config.external_skills.auto_expose_installed);
    }

    #[test]
    fn from_env_parses_default_allowlist() {
        // When the env var is not set, from_env falls back to the hardcoded
        // defaults: echo, pwd.
        let config = ToolRuntimeConfig::from_env();
        // We can't guarantee the env var is unset in all CI environments,
        // but the parser itself should produce a non-empty set either way.
        assert!(!config.shell_allowlist.is_empty());
    }

    #[cfg(feature = "tool-shell")]
    #[test]
    fn injected_config_overrides_global() {
        let config = ToolRuntimeConfig {
            shell_allowlist: BTreeSet::from(["echo".to_owned()]),
            file_root: Some(PathBuf::from("/tmp/injected-root")),
            external_skills: ExternalSkillsRuntimePolicy::default(),
            ..ToolRuntimeConfig::default()
        };
        let result = crate::tools::execute_tool_core_with_config(
            loongclaw_contracts::ToolCoreRequest {
                tool_name: "shell.exec".to_owned(),
                payload: serde_json::json!({"command": "echo", "args": ["injected"]}),
            },
            &config,
        );
        let outcome = result.expect("echo should be allowed with injected config");
        assert_eq!(outcome.status, "ok");
        assert!(
            outcome.payload["stdout"]
                .as_str()
                .unwrap()
                .contains("injected")
        );
    }

    #[test]
    fn from_env_parses_external_skills_policy() {
        crate::process_env::set_var("LOONGCLAW_EXTERNAL_SKILLS_ENABLED", "true");
        crate::process_env::set_var(
            "LOONGCLAW_EXTERNAL_SKILLS_REQUIRE_DOWNLOAD_APPROVAL",
            "false",
        );
        crate::process_env::set_var(
            "LOONGCLAW_EXTERNAL_SKILLS_ALLOWED_DOMAINS",
            "skills.sh,clawhub.io",
        );
        crate::process_env::set_var(
            "LOONGCLAW_EXTERNAL_SKILLS_BLOCKED_DOMAINS",
            "malicious.example",
        );
        crate::process_env::set_var(
            "LOONGCLAW_EXTERNAL_SKILLS_INSTALL_ROOT",
            "/tmp/managed-skills",
        );
        crate::process_env::set_var("LOONGCLAW_EXTERNAL_SKILLS_AUTO_EXPOSE_INSTALLED", "false");

        let config = ToolRuntimeConfig::from_env();
        assert!(config.external_skills.enabled);
        assert!(!config.external_skills.require_download_approval);
        assert!(config.external_skills.allowed_domains.contains("skills.sh"));
        assert!(
            config
                .external_skills
                .allowed_domains
                .contains("clawhub.io")
        );
        assert!(
            config
                .external_skills
                .blocked_domains
                .contains("malicious.example")
        );
        assert_eq!(
            config.external_skills.install_root,
            Some(PathBuf::from("/tmp/managed-skills"))
        );
        assert!(!config.external_skills.auto_expose_installed);

        crate::process_env::remove_var("LOONGCLAW_EXTERNAL_SKILLS_ENABLED");
        crate::process_env::remove_var("LOONGCLAW_EXTERNAL_SKILLS_REQUIRE_DOWNLOAD_APPROVAL");
        crate::process_env::remove_var("LOONGCLAW_EXTERNAL_SKILLS_ALLOWED_DOMAINS");
        crate::process_env::remove_var("LOONGCLAW_EXTERNAL_SKILLS_BLOCKED_DOMAINS");
        crate::process_env::remove_var("LOONGCLAW_EXTERNAL_SKILLS_INSTALL_ROOT");
        crate::process_env::remove_var("LOONGCLAW_EXTERNAL_SKILLS_AUTO_EXPOSE_INSTALLED");
    }

    #[test]
    fn external_skills_policy_struct_construction() {
        let policy = ExternalSkillsRuntimePolicy {
            enabled: true,
            require_download_approval: false,
            allowed_domains: BTreeSet::from(["skills.sh".to_owned(), "clawhub.io".to_owned()]),
            blocked_domains: BTreeSet::from(["malicious.example".to_owned()]),
            install_root: Some(PathBuf::from("/tmp/managed-skills")),
            auto_expose_installed: false,
        };

        assert!(policy.enabled);
        assert!(!policy.require_download_approval);
        assert!(policy.allowed_domains.contains("skills.sh"));
        assert!(policy.allowed_domains.contains("clawhub.io"));
        assert!(policy.blocked_domains.contains("malicious.example"));
        assert_eq!(
            policy.install_root,
            Some(PathBuf::from("/tmp/managed-skills"))
        );
        assert!(!policy.auto_expose_installed);
    }

    #[cfg(feature = "feishu-integration")]
    #[test]
    fn from_env_enables_feishu_runtime_when_credentials_exist() {
        crate::process_env::set_var("FEISHU_APP_ID", "cli_env_a1b2c3");
        crate::process_env::set_var("FEISHU_APP_SECRET", "env-secret");

        let config = ToolRuntimeConfig::from_env();
        let feishu = config
            .feishu
            .as_ref()
            .expect("feishu runtime should be enabled from env");

        assert!(feishu.channel.enabled);
        assert_eq!(feishu.channel.app_id_env.as_deref(), Some("FEISHU_APP_ID"));
        assert_eq!(
            feishu.channel.app_secret_env.as_deref(),
            Some("FEISHU_APP_SECRET")
        );
        assert_eq!(
            feishu.integration.resolved_sqlite_path(),
            crate::config::default_loongclaw_home().join("feishu.sqlite3")
        );

        crate::process_env::remove_var("FEISHU_APP_ID");
        crate::process_env::remove_var("FEISHU_APP_SECRET");
    }

    #[cfg(feature = "feishu-integration")]
    #[test]
    fn from_loongclaw_config_ignores_disabled_feishu_channel_even_when_root_credentials_exist() {
        let config = crate::config::LoongClawConfig {
            feishu: crate::config::FeishuChannelConfig {
                enabled: false,
                app_id: Some("cli_disabled_root".to_owned()),
                app_secret: Some("disabled-root-secret".to_owned()),
                ..crate::config::FeishuChannelConfig::default()
            },
            ..crate::config::LoongClawConfig::default()
        };

        assert!(
            FeishuToolRuntimeConfig::from_loongclaw_config(&config).is_none(),
            "disabled Feishu channel should not expose Feishu tools through runtime config"
        );
    }

    #[cfg(feature = "feishu-integration")]
    #[test]
    fn from_loongclaw_config_ignores_disabled_feishu_accounts_when_detecting_runtime() {
        let config = crate::config::LoongClawConfig {
            feishu: crate::config::FeishuChannelConfig {
                enabled: true,
                accounts: BTreeMap::from([(
                    "disabled_account".to_owned(),
                    crate::config::FeishuAccountConfig {
                        enabled: Some(false),
                        app_id: Some("cli_disabled_account".to_owned()),
                        app_secret: Some("disabled-account-secret".to_owned()),
                        ..crate::config::FeishuAccountConfig::default()
                    },
                )]),
                ..crate::config::FeishuChannelConfig::default()
            },
            ..crate::config::LoongClawConfig::default()
        };

        assert!(
            FeishuToolRuntimeConfig::from_loongclaw_config(&config).is_none(),
            "disabled Feishu accounts should not enable Feishu tool runtime on their own"
        );
    }
}
