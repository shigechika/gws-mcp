---
"@googleworkspace/cli": patch
---

fix(gmail): include RFC 2047-encoded From header in gmail_reply MCP tool output.

The `gmail_reply` MCP tool was building the raw MIME message without a From
header, causing Gmail to inject the sender's display name as raw UTF-8 bytes.
Mail clients that do not support SMTPUTF8 interpreted those bytes as Latin-1,
producing mojibake for accounts whose display name contains non-ASCII characters
(e.g. Japanese, Chinese, accented Latin).

Fix: call `resolve_sender` in `mcp_compose_reply` so that the From header is
always present and properly RFC 2047-encoded, matching the behaviour of the CLI
`gmail +reply` subcommand.
