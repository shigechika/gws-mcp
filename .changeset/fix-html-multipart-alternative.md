---
"@googleworkspace/cli": patch
---

fix(gmail): emit multipart/alternative for HTML messages. HTML-only email (text/html with no text/plain part) is flagged by SpamAssassin MIME_HTML_ONLY and unreadable in plain-text clients. gmail_send and gmail_reply now build a multipart/alternative container with a tag-stripped text/plain fallback alongside the HTML part, following RFC 2046 §5.1.4 ordering.
