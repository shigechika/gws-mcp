---
"@googleworkspace/cli": patch
---

fix(mcp): map loopback binds to localhost in server_base

Both IPv4 (`127.0.0.1`, `0.0.0.0`) and IPv6 (`::`, `::1`) loopback
binds now advertise `http://localhost:<port>` in the RFC 9728 Protected
Resource Metadata. This ensures the `resource` URL matches the canonical
URL clients use to connect, fixing MCP Authorization spec validation
failures in Claude Code:

  SDK auth failed: Protected resource http://127.0.0.1:3000 does not
  match expected http://localhost:3000/mcp (or origin)
