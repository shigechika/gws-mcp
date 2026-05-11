---
"@googleworkspace/cli": patch
---

fix(auth): use absolute URL for resource_metadata in WWW-Authenticate header. RFC 9728 §5.1 requires resource_metadata to be an absolute HTTPS URL. The previous hardcoded path (`/.well-known/oauth-protected-resource`) was resolved relative to the server root by MCP clients, causing OAuth discovery to fail when the server is mounted at a sub-path (e.g. `--public-url https://mcp.example.com/gws`). The URL is now constructed from `state.base_url` set by `--public-url`.
