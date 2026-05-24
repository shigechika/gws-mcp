---
"@googleworkspace/cli": patch
---

fix(auth): invalidate token_cache.json after re-login to a different account. Previously the stale cached access token persisted for up to ~1 hour after switching accounts, causing all API calls to silently use the old account's credentials. The cache file is now deleted immediately after new credentials are saved successfully.
