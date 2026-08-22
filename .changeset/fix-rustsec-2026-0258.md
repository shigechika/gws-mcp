---
"@googleworkspace/cli": patch
---

chore(deps): bump `h2` to 0.4.18 to resolve RUSTSEC-2026-0258 (unbounded empty DATA frames could cause unbounded memory usage or a panic). No behavior change; fixes the `Cargo Deny`/`Audit` CI checks, which had been failing on `main` since the advisory was published.
