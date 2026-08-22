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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use yup_oauth2::storage::{TokenInfo, TokenStorage, TokenStorageError};

use crate::output::sanitize_for_terminal;

/// A custom token storage implementation for `yup-oauth2` that encrypts
/// the cached tokens at rest using AES-256-GCM encryption.
pub struct EncryptedTokenStorage {
    file_path: PathBuf,
    /// Opaque per-account identifier mixed into cache keys so tokens for
    /// different accounts (e.g. when switching via
    /// `GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE`) never collide on scopes alone.
    /// Empty string means "no account namespacing" (legacy behavior).
    account_id: String,
    // Add memory cache since TokenStorage getters can be called frequently
    cache: Arc<Mutex<Option<HashMap<String, TokenInfo>>>>,
}

impl EncryptedTokenStorage {
    pub fn new(path: PathBuf) -> Self {
        Self::with_account(path, String::new())
    }

    /// Like [`EncryptedTokenStorage::new`] but namespaces every cache entry by
    /// `account_id`, so the same cache file can hold tokens for multiple
    /// accounts without one account's token being served for another.
    pub fn with_account(path: PathBuf, account_id: String) -> Self {
        Self {
            file_path: path,
            account_id,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    async fn load_from_disk(&self) -> HashMap<String, TokenInfo> {
        let data = match tokio::fs::read(&self.file_path).await {
            Ok(d) => d,
            Err(_) => return HashMap::new(), // File doesn't exist yet — normal on first run
        };

        let decrypted = match crate::credential_store::decrypt(&data) {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "warning: failed to decrypt token cache ({}): {e:#}",
                    self.file_path.display()
                );
                eprintln!("hint: you may need to re-authenticate with `gws auth login`");
                return HashMap::new();
            }
        };

        let json = match String::from_utf8(decrypted) {
            Ok(j) => j,
            Err(e) => {
                eprintln!(
                    "warning: token cache contains invalid UTF-8: {}",
                    sanitize_for_terminal(&e.to_string())
                );
                return HashMap::new();
            }
        };

        match serde_json::from_str(&json) {
            Ok(map) => map,
            Err(e) => {
                eprintln!(
                    "warning: failed to parse token cache JSON: {}",
                    sanitize_for_terminal(&e.to_string())
                );
                HashMap::new()
            }
        }
    }

    async fn save_to_disk(&self, map: &HashMap<String, TokenInfo>) -> anyhow::Result<()> {
        let json = serde_json::to_string(map)?;
        let encrypted = crate::credential_store::encrypt(json.as_bytes())?;

        if let Some(parent) = self.file_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                anyhow::anyhow!(
                    "Failed to create token directory '{}': {}",
                    sanitize_for_terminal(&parent.display().to_string()),
                    e
                )
            })?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                tokio::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to set permissions on token directory '{}': {}",
                            sanitize_for_terminal(&parent.display().to_string()),
                            e
                        )
                    })?;
            }
        }

        // Write atomically via a sibling .tmp file + rename.
        crate::fs_util::atomic_write_async(&self.file_path, encrypted.as_slice()).await?;

        Ok(())
    }

    // Helper to join scopes consistently for cache keys, namespaced by account.
    fn cache_key(&self, scopes: &[&str]) -> String {
        let mut s: Vec<&str> = scopes.to_vec();
        s.sort_unstable();
        s.dedup();
        let scope_part = s.join(" ");
        if self.account_id.is_empty() {
            scope_part
        } else {
            // Unit separator (0x1f) can't appear in scope URLs, so the account
            // prefix never collides with scope content.
            format!("{}\u{1f}{}", self.account_id, scope_part)
        }
    }
}

#[async_trait::async_trait]
impl TokenStorage for EncryptedTokenStorage {
    async fn set(&self, scopes: &[&str], token: TokenInfo) -> Result<(), TokenStorageError> {
        let mut map_lock = self.cache.lock().await;

        // Initialize cache if this is the first write
        if map_lock.is_none() {
            *map_lock = Some(self.load_from_disk().await);
        }

        if let Some(map) = map_lock.as_mut() {
            map.insert(self.cache_key(scopes), token);
            self.save_to_disk(map)
                .await
                .map_err(|e| TokenStorageError::Other(std::borrow::Cow::Owned(e.to_string())))?;
        }

        Ok(())
    }

    async fn get(&self, scopes: &[&str]) -> Option<TokenInfo> {
        let mut map_lock = self.cache.lock().await;

        if map_lock.is_none() {
            *map_lock = Some(self.load_from_disk().await);
        }

        if let Some(map) = map_lock.as_ref() {
            let key = self.cache_key(scopes);
            if let Some(token) = map.get(&key) {
                let mut token = token.clone();
                if token.expires_at.is_none() {
                    // yup-oauth2's is_expired() treats a missing expires_at as "not
                    // expired", so a legacy entry would be served forever; backdate it
                    // to force a refresh, which repopulates a real expiry (upstream #904).
                    token.expires_at = Some(time::OffsetDateTime::UNIX_EPOCH);
                }
                return Some(token);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_encrypted_token_storage_new() {
        let path = PathBuf::from("/fake/path/to/token.json");
        let storage = EncryptedTokenStorage::new(path.clone());

        assert_eq!(storage.file_path, path);
        assert!(storage.account_id.is_empty());

        let cache_lock = storage.cache.lock().await;
        assert!(cache_lock.is_none());
    }

    #[test]
    fn test_cache_key_without_account_is_scopes_only() {
        let storage = EncryptedTokenStorage::new(PathBuf::from("/x"));
        let key = storage.cache_key(&["b.scope", "a.scope"]);
        // Sorted, deduped, space-joined, no account prefix.
        assert_eq!(key, "a.scope b.scope");
    }

    #[test]
    fn test_cache_key_namespaced_by_account() {
        let scopes = ["https://www.googleapis.com/auth/gmail.readonly"];
        let a = EncryptedTokenStorage::with_account(PathBuf::from("/x"), "acct-a".to_string());
        let b = EncryptedTokenStorage::with_account(PathBuf::from("/x"), "acct-b".to_string());
        let key_a = a.cache_key(&scopes);
        let key_b = b.cache_key(&scopes);

        // Same scopes, different accounts → distinct cache keys (upstream #572).
        assert_ne!(key_a, key_b);
        assert!(key_a.starts_with("acct-a"));
        assert!(key_a.ends_with(scopes[0]));
    }

    #[tokio::test]
    async fn test_account_namespacing_isolates_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("token_cache.json");
        let scopes = ["https://www.googleapis.com/auth/gmail.readonly"];

        let account_a =
            EncryptedTokenStorage::with_account(cache_path.clone(), "account-a".to_string());
        let token = TokenInfo {
            access_token: Some("token-for-a".to_string()),
            refresh_token: None,
            expires_at: None,
            id_token: None,
        };
        account_a.set(&scopes, token).await.unwrap();

        // A fresh storage for a different account, same file + same scopes,
        // must NOT see account A's token (the #572 regression).
        let account_b =
            EncryptedTokenStorage::with_account(cache_path.clone(), "account-b".to_string());
        assert!(account_b.get(&scopes).await.is_none());

        // The same account still reads its own token back (new instance to
        // force a fresh disk load rather than reusing the in-memory cache).
        let account_a_again =
            EncryptedTokenStorage::with_account(cache_path, "account-a".to_string());
        let got = account_a_again.get(&scopes).await.unwrap();
        assert_eq!(got.access_token.as_deref(), Some("token-for-a"));
    }

    #[tokio::test]
    async fn test_get_backdates_missing_expiry_to_force_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("token_cache.json");
        let scopes = ["https://www.googleapis.com/auth/gmail.readonly"];

        let storage = EncryptedTokenStorage::new(cache_path);
        let token = TokenInfo {
            access_token: Some("stale-access-token".to_string()),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: None,
            id_token: None,
        };
        storage.set(&scopes, token).await.unwrap();

        let got = storage.get(&scopes).await.unwrap();
        assert!(got.is_expired(), "missing expires_at must read as expired");
        assert_eq!(got.refresh_token.as_deref(), Some("refresh-token"));
    }

    #[tokio::test]
    async fn test_get_preserves_real_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("token_cache.json");
        let scopes = ["https://www.googleapis.com/auth/gmail.readonly"];

        let future = time::OffsetDateTime::now_utc() + time::Duration::HOUR;
        let storage = EncryptedTokenStorage::new(cache_path);
        let token = TokenInfo {
            access_token: Some("fresh-access-token".to_string()),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: Some(future),
            id_token: None,
        };
        storage.set(&scopes, token.clone()).await.unwrap();

        let got = storage.get(&scopes).await.unwrap();
        assert_eq!(got, token);
        assert!(!got.is_expired());
    }
}
