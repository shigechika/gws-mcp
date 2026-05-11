---
"@googleworkspace/cli": patch
---

fix(auth): support both "installed" and "web" client_secret.json formats. `load_client_config()` previously only accepted the `"installed"` (Desktop app) key. Web Application type OAuth clients — required for HTTPS redirect URIs in reverse-proxy deployments (`--public-url`) — use the `"web"` key instead. Both keys are now accepted; `"installed"` takes precedence when both are present.
