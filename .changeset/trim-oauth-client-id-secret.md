---
"@googleworkspace/cli": patch
---

fix(auth): trim whitespace from the OAuth Client ID/Secret entered in `gws auth setup`, and from `client_secret.json` on load. A trailing space (easy to pick up copying from the Cloud Console credentials page) previously produced a misleading `401 invalid_client` at login instead of a clear validation error (upstream #882). Trimming on load also self-heals a value persisted before this fix, without re-running setup.
