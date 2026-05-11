// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Helpers for the OAuth client configuration file.
//!
//! Supports both Google Cloud Console download formats:
//!
//! Desktop / installed app (`"installed"` key):
//! ```json
//! {
//!   "installed": {
//!     "client_id": "...apps.googleusercontent.com",
//!     "client_secret": "GOCSPX-...",
//!     "project_id": "my-project",
//!     "auth_uri": "https://accounts.google.com/o/oauth2/auth",
//!     "token_uri": "https://oauth2.googleapis.com/token",
//!     "redirect_uris": ["http://localhost"]
//!   }
//! }
//! ```
//!
//! Web application (`"web"` key):
//! ```json
//! {
//!   "web": {
//!     "client_id": "...apps.googleusercontent.com",
//!     "client_secret": "GOCSPX-...",
//!     "project_id": "my-project",
//!     "auth_uri": "https://accounts.google.com/o/oauth2/auth",
//!     "token_uri": "https://oauth2.googleapis.com/token",
//!     "redirect_uris": ["https://example.com/oauth/callback"]
//!   }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The "installed" application config from Google Cloud Console.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstalledConfig {
    pub client_id: String,
    pub client_secret: String,
    pub project_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    #[serde(default)]
    pub auth_provider_x509_cert_url: String,
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

/// Wrapper matching the Google Cloud Console download format.
/// Supports both `"installed"` (desktop) and `"web"` (web application) client types.
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientSecretFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed: Option<InstalledConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<InstalledConfig>,
}

/// Returns the path for the client secret config file.
pub fn client_config_path() -> PathBuf {
    crate::auth_commands::config_dir().join("client_secret.json")
}

/// Saves OAuth client configuration in the standard Google Cloud Console format.
pub fn save_client_config(
    client_id: &str,
    client_secret: &str,
    project_id: &str,
) -> anyhow::Result<PathBuf> {
    let config = ClientSecretFile {
        installed: Some(InstalledConfig {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            project_id: project_id.to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs".to_string(),
            redirect_uris: vec!["http://localhost".to_string()],
        }),
        web: None,
    };

    let path = client_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&config)?;
    crate::fs_util::atomic_write(&path, json.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to write client config: {e}"))?;

    Ok(path)
}

/// Loads OAuth client configuration from the standard Google Cloud Console format.
/// Accepts both `"installed"` (desktop) and `"web"` (web application) client types.
pub fn load_client_config() -> anyhow::Result<InstalledConfig> {
    let path = client_config_path();
    let data = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Cannot read {}: {e}", path.display()))?;
    let file: ClientSecretFile = serde_json::from_str(&data)
        .map_err(|e| anyhow::anyhow!("Invalid client_secret.json format: {e}"))?;
    file.installed.or(file.web).ok_or_else(|| {
        anyhow::anyhow!("client_secret.json must contain an 'installed' or 'web' key (got neither)")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("client_secret.json");

        let config = ClientSecretFile {
            installed: Some(InstalledConfig {
                client_id: "test-id.apps.googleusercontent.com".to_string(),
                client_secret: "GOCSPX-test".to_string(),
                project_id: "my-project".to_string(),
                auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
                token_uri: "https://oauth2.googleapis.com/token".to_string(),
                auth_provider_x509_cert_url: "https://www.googleapis.com/oauth2/v1/certs"
                    .to_string(),
                redirect_uris: vec!["http://localhost".to_string()],
            }),
            web: None,
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        std::fs::write(&path, &json).unwrap();

        let data = std::fs::read_to_string(&path).unwrap();
        let loaded: ClientSecretFile = serde_json::from_str(&data).unwrap();

        let cfg = loaded.installed.unwrap();
        assert_eq!(cfg.client_id, "test-id.apps.googleusercontent.com");
        assert_eq!(cfg.client_secret, "GOCSPX-test");
        assert_eq!(cfg.project_id, "my-project");
    }

    #[test]
    fn test_parse_installed_format() {
        let json = r#"{
            "installed": {
                "client_id": "test-client-id.apps.googleusercontent.com",
                "project_id": "test-project-id",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
                "client_secret": "test-client-secret",
                "redirect_uris": ["http://localhost"]
            }
        }"#;

        let config: ClientSecretFile = serde_json::from_str(json).unwrap();
        let cfg = config.installed.unwrap();
        assert_eq!(cfg.project_id, "test-project-id");
        assert_eq!(cfg.client_secret, "test-client-secret");
        assert_eq!(cfg.redirect_uris, vec!["http://localhost"]);
        assert!(config.web.is_none());
    }

    #[test]
    fn test_parse_web_format() {
        // Web application format from Google Cloud Console
        let json = r#"{
            "web": {
                "client_id": "web-client-id.apps.googleusercontent.com",
                "project_id": "my-project",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
                "client_secret": "web-client-secret",
                "redirect_uris": ["https://mcp.example.com/gws/oauth/callback"]
            }
        }"#;

        let config: ClientSecretFile = serde_json::from_str(json).unwrap();
        assert!(config.installed.is_none());
        let cfg = config.web.unwrap();
        assert_eq!(cfg.client_id, "web-client-id.apps.googleusercontent.com");
        assert_eq!(cfg.client_secret, "web-client-secret");
        assert_eq!(
            cfg.redirect_uris,
            vec!["https://mcp.example.com/gws/oauth/callback"]
        );
    }

    #[test]
    fn test_load_client_config_prefers_installed_over_web() {
        // If both keys exist (unusual), "installed" takes precedence
        let json = r#"{
            "installed": {
                "client_id": "installed-id",
                "project_id": "p",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "client_secret": "installed-secret"
            },
            "web": {
                "client_id": "web-id",
                "project_id": "p",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "client_secret": "web-secret"
            }
        }"#;

        let file: ClientSecretFile = serde_json::from_str(json).unwrap();
        let cfg = file.installed.or(file.web).unwrap();
        assert_eq!(cfg.client_id, "installed-id");
    }

    #[test]
    fn test_parse_missing_optional_fields() {
        let json = r#"{
            "installed": {
                "client_id": "test-id",
                "project_id": "test-project",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token",
                "client_secret": "secret"
            }
        }"#;

        let config: ClientSecretFile = serde_json::from_str(json).unwrap();
        let cfg = config.installed.unwrap();
        assert_eq!(cfg.client_id, "test-id");
        assert!(cfg.redirect_uris.is_empty());
        assert!(cfg.auth_provider_x509_cert_url.is_empty());
    }

    #[test]
    fn test_parse_neither_key_fails_at_load() {
        // Parses OK as JSON but load_client_config() would return Err
        let json = r#"{ "wrong_key": {} }"#;
        let file: ClientSecretFile = serde_json::from_str(json).unwrap();
        assert!(file.installed.is_none());
        assert!(file.web.is_none());
        let result = file.installed.or(file.web);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_invalid_json_fails() {
        let json = r#"not json"#;
        let result = serde_json::from_str::<ClientSecretFile>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_client_id_fails() {
        let json = r#"{
            "installed": {
                "project_id": "test",
                "auth_uri": "https://example.com",
                "token_uri": "https://example.com",
                "client_secret": "secret"
            }
        }"#;
        let result = serde_json::from_str::<ClientSecretFile>(json);
        assert!(result.is_err());
    }

    // Helper to manage the env var safely and clean up automatically
    struct EnvGuard {
        key: String,
        original_value: Option<String>,
    }

    impl EnvGuard {
        fn new(key: &str, value: &str) -> Self {
            let original_value = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self {
                key: key.to_string(),
                original_value,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(val) = &self.original_value {
                std::env::set_var(&self.key, val);
            } else {
                std::env::remove_var(&self.key);
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_load_client_config() {
        let dir = tempfile::tempdir().unwrap();
        let _env_guard = EnvGuard::new(
            "GOOGLE_WORKSPACE_CLI_CONFIG_DIR",
            dir.path().to_str().unwrap(),
        );

        // Initially no config file exists
        let result = load_client_config();
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Cannot read"));

        // Create a valid config file
        save_client_config("test-id", "test-secret", "test-project").unwrap();

        // Now loading should succeed
        let config = load_client_config().unwrap();
        assert_eq!(config.client_id, "test-id");
        assert_eq!(config.client_secret, "test-secret");
        assert_eq!(config.project_id, "test-project");

        // Create an invalid config file
        let path = client_config_path();
        std::fs::write(&path, "invalid json").unwrap();

        let result = load_client_config();
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("Invalid client_secret.json format"));
    }
}
