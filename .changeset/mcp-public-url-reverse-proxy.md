---
"@googleworkspace/cli": minor
---

feat(mcp): add --public-url for reverse proxy deployments

Adds `--public-url <URL>` to `gws mcp --transport http --auth` so the
server can be deployed behind a reverse proxy (e.g. Caddy, nginx) with
TLS termination. Without this option, the RFC 9728 / RFC 8414 OAuth2
metadata always advertised `http://localhost:<port>`, which is
unreachable to mobile MCP clients connecting via the public URL.

The value overrides `server_base()` in all OAuth2 discovery documents
(`resource`, `authorization_servers`, `issuer`, `authorization_endpoint`,
`token_endpoint`, `registration_endpoint`) and in the Google redirect URI.
Validation: http/https scheme only, no query/fragment, trailing slash
stripped. Plain http:// with a non-loopback host emits a warning per
RFC 9728 §3.1.

Closes #24.
