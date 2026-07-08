---
"@googleworkspace/cli": patch
---

Fix `encode_path_segment` percent-encoding RFC 3986 unreserved characters (`-`, `.`, `_`, `~`) in URL path segments (file IDs, calendar IDs, message IDs, script IDs, etc.). Previously every non-alphanumeric character was encoded, so e.g. an Apps Script `scriptId` containing `-` or `_` (very common — script IDs are base64url-derived) got mangled into `%2D`/`%5F`, and the Apps Script Execution API rejected the resulting URL with 404 (upstream #842). Only characters that can actually alter URL structure are encoded now; unreserved characters are left intact.
