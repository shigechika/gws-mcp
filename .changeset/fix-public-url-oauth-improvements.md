---
"@googleworkspace/cli": patch
---

fix(auth): OAuth improvements for public HTTPS deployments behind a reverse proxy.

1. Allow `https://` redirect URIs when `--public-url https://...` is set. Previously, `/oauth/authorize` restricted `redirect_uri` to loopback addresses only, causing remote MCP clients (e.g. those using `https://` callbacks) to receive `400 Bad Request`.

2. Disable `allowed_hosts` checking when `--public-url` is set. A TLS reverse proxy forwards requests with the public hostname in the `Host` header, which caused the default `StreamableHttpServerConfig` allowed_hosts check to reject all authenticated MCP requests with `403`.
