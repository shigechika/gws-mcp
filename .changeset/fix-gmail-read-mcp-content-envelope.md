---
"@googleworkspace/cli": patch
---

fix(mcp): wrap `gmail_read` result in the MCP content envelope.

`gmail_read` returned the bare data object instead of the
`{ "content": [{ "type": "text", "text": ... }], "isError": false }` envelope
that MCP clients render, so the tool call completed with no visible output.
The structured JSON is now pretty-printed into a single text block (matching
`gmail_send`/`gmail_reply`).
