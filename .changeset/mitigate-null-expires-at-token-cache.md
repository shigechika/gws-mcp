---
"@googleworkspace/cli": patch
---

fix(auth): treat token cache entries with a missing `expires_at` as expired instead of valid forever. `yup-oauth2`'s `is_expired()` defaults a `None` expiry to "not expired", so a legacy or corrupted cache entry with no expiry was served forever and never refreshed (upstream #904). `EncryptedTokenStorage::get()` now backdates a missing `expires_at` to the Unix epoch before returning it, forcing the refresh flow — which preserves the `refresh_token` and repopulates a real `expires_at` on success, so the entry self-heals after one refresh.
