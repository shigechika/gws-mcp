---
"@googleworkspace/cli": patch
---

chore(deps): bump `anyhow` to 1.0.103, `serial_test` to 3.5.0 (dropping its transitive `scc` dependency), and `quinn-proto` to 0.11.16 to resolve RUSTSEC-2026-0190 (anyhow unsoundness), RUSTSEC-2026-0205 (scc unsoundness), and RUSTSEC-2026-0185 (quinn-proto remote memory exhaustion). No behavior change; fixes the `Audit` CI workflow, which had been failing on `main` since these advisories were published.
