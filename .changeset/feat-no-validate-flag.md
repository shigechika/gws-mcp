---
"@googleworkspace/cli": patch
---

feat: add a global `--no-validate` flag that skips the local request-body check against the Discovery schema, so fields the live API accepts but Discovery does not yet describe (e.g. Developer Preview surfaces) can be sent (upstream #901). CLI-only: MCP tools and helpers keep validation on.
