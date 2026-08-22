---
"@googleworkspace/cli": patch
---

fix(auth): when `credentials.enc` fails to decrypt (e.g. after a keyring/encryption-key change or a Keychain/keyring backend becoming unreachable), it is now renamed to `credentials.enc.unreadable.<timestamp>` instead of silently deleted (upstream #886). The timestamp suffix means a second, later decrypt failure can't silently clobber an earlier failure's preserved file — plain rename overwrites an existing destination without warning. The actual decrypt error is printed, along with the rename target on success or a note that the file remains at its original path on rename failure. Token cache files (`token_cache.json`, `sa_token_cache.json`) are still deleted outright, since they're re-derivable from a fresh `gws auth login` — only the credential file itself is preserved.
