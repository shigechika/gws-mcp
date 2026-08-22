---
"@googleworkspace/cli": patch
---

fix(auth): warn when the quota project silently falls back to ambient gcloud Application Default Credentials (upstream #878). With no `GOOGLE_WORKSPACE_PROJECT_ID` and no `project_id` in `client_secret.json`, `get_quota_project()` reads `quota_project_id` from `~/.config/gcloud/application_default_credentials.json`, which lives outside any `GOOGLE_WORKSPACE_CLI_CONFIG_DIR` binding and can point at an unrelated project. The fallback behavior is unchanged; a warning is now printed to stderr naming the ADC project used, so a resulting 403 against an unfamiliar project isn't a mystery.
