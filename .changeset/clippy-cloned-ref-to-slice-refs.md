---
"@googleworkspace/cli": patch
---

Fix new `clippy::cloned_ref_to_slice_refs` lint failures in `mcp_server.rs` (clippy 1.97+) by replacing `&[svc_name.to_string()]` with `std::slice::from_ref(svc_name)`. No behavior change; this had been failing the post-merge `ci.yml` lint job on `main` for several weeks because recent PRs only touched docs/CI files and skipped the Lint job via `ci.yml`'s change-detection gate.
