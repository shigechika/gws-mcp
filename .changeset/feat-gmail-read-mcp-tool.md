---
"@googleworkspace/cli": minor
---

feat(mcp): add `gmail_read` helper tool.

Exposes the existing `gmail +read` logic as an MCP helper tool (enabled with
`--helpers`). It fetches a message and returns its parsed headers and decoded
body as compact JSON — extracting the plain-text body (handling
multipart/alternative, base64, and HTML-to-text conversion) instead of the raw
API payload. This keeps MIME boundaries, base64 blobs, and DKIM/ARC headers out
of the agent's context window, addressing the motivation behind upstream
googleworkspace/cli#438. Supports `html` (return the HTML body) and
`include_headers` (default true), and is annotated `readOnlyHint: true`.
