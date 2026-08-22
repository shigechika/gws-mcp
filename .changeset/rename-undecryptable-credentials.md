---
"@googleworkspace/cli": patch
---

fix(auth): when `credentials.enc` fails to decrypt (e.g. after a keyring/encryption-key change or a Keychain/keyring backend becoming unreachable), it is now renamed to `credentials.enc.unreadable` instead of silently deleted (upstream #886). The actual decrypt error and the rename target are printed, so the cause is visible and the file is recoverable for inspection instead of vanishing with only a warning. Token cache files (`token_cache.json`, `sa_token_cache.json`) are still deleted outright, since they're re-derivable from a fresh `gws auth login` — only the credential file itself is preserved.
