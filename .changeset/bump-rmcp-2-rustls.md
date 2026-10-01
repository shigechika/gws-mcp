---
"@googleworkspace/cli": patch
---

fix(deps): bump rmcp 1.6 → 2.2 and rustls 0.23.39 → 0.23.45. Resolves six rmcp Dependabot alerts in the Streamable HTTP transport (unauthenticated session-table leak, missing `resource` validation in OAuth protected-resource metadata, custom headers leaking to cross-origin redirects) and RUSTSEC-2026-0285 (TLS 1.3 handshake messages accepted across encryption levels), which was failing `cargo deny` on main. No source changes were needed.
