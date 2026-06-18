---
"@googleworkspace/cli": patch
---

fix(auth): key the token cache by account, not just scopes.

When switching accounts via `GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE` (or any
credential source) while requesting the same scopes, the encrypted token cache
served the previously cached account's access token, because cache entries were
keyed by scopes alone (upstream googleworkspace/cli#572).

Fix: `EncryptedTokenStorage` now namespaces every cache entry by a stable,
non-reversible account fingerprint (a short SHA-256 prefix over the credential's
identity — refresh token for authorized users, client email + private key id for
service accounts). The underlying secrets are never stored in the cache in
recoverable form. Tokens for different accounts no longer collide on scopes.
