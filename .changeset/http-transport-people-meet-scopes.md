---
"@googleworkspace/cli": patch
---

fix(auth): the `--auth` HTTP transport never requested People or Meet API scopes, so `people`/`meet` tool calls over that transport always failed with insufficient-scope errors (upstream #556). `gws_scopes_for_services` derives scopes statically from a candidate list, and while `map_service_to_scope_prefixes` already mapped `people`/`meet` to their scope prefixes, no candidate scopes existed to match against. Added a dedicated candidate list (`HTTP_TRANSPORT_EXTRA_SCOPES`: `contacts.readonly`, `meetings.space.created`) used only by that function, so `gws auth login`'s own default scopes (`MINIMAL_SCOPES`/`DEFAULT_SCOPES`) are unaffected — the interactive CLI scope picker already discovers scopes dynamically via Discovery documents and didn't have this bug.
