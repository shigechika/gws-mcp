When contributing to this repository, you must strictly follow all guidelines outlined in the AGENTS.md file.

## Fork-specific Notes

This is a fork of `googleworkspace/cli` that maintains MCP server support. See `FORK.md` for details.

### Upstream Merge Checklist

After merging upstream/main, fix MCP compilation errors:
1. `crates/google-workspace-cli/src/mcp_server.rs` — match new arguments in `executor::execute_method()` calls
2. `crates/google-workspace-cli/src/mcp_server.rs` — match new fields in Gmail helper structs
3. `crates/google-workspace-cli/src/helpers/gmail/mod.rs` — ensure `pub(crate)` visibility is not reverted to `pub(super)`
4. `pub(crate)` targets: `Mailbox`, `to_mb_address_list`, `apply_optional_headers`, `finalize_message`, `resolve_mail_method`, `Attachment`
5. Run `cargo clippy -- -D warnings && cargo test` to verify
6. If conflicts exceed ~20 files, consider rebasing: checkout upstream/main as new branch, re-apply MCP changes, reset main

### Project Structure

- Cargo workspace: `crates/google-workspace-cli/` (binary) + `crates/google-workspace/` (library)
- MCP server (stdio transport): `crates/google-workspace-cli/src/mcp_server.rs`. The fork-only **HTTP transport** — Streamable HTTP plus an embedded OAuth 2.1 authorization/resource server (per-user Google token isolation) — lives in `crates/google-workspace-cli/src/mcp_http_server.rs` and is arguably the fork's largest security surface; keep it in scope when reviewing auth/MCP changes.
- Local install: `cargo install --path crates/google-workspace-cli`

### GitHub Actions

- Fork workflows (this list grows over time): `ci.yml`, `policy.yml`, `sync-upstream.yml`, `audit.yml`, `auto-tag.yml`, `release-fork.yml`
- `gh workflow list` may show upstream workflows — use `gh api repos/<owner>/<repo>/actions/workflows` to check fork workflows, but confirm the file actually exists on the **default branch**: that API keeps reporting a workflow as "active" while its file only survives on stale side branches (this is how the deleted `generate-skills.yml` kept getting re-added to docs)
- `gh run list` may show upstream runs — filter with `--branch=main` for fork-specific results
- `audit.yml` runs `cargo audit` daily (06:00 UTC cron) plus on `Cargo.toml`/`Cargo.lock` pushes/PRs. When it fails on a new RUSTSEC advisory, `cargo update -p <crate>` (or a transitive dep swap, e.g. bumping `serial_test` dropped its vulnerable `scc` dependency) is usually enough — no source changes needed, just a `Cargo.lock`-only PR with a changeset.
- `ci.yml` has a **Verify Skills** job that reruns `cargo run -- generate-skills --output-dir skills` and fails on any drift, plus a **Lint Skills** job that validates `skills/` with `agentskills`. A change to MCP tool schemas or `generate_skills.rs` must therefore commit a regenerated `skills/` directory in the same PR — CI will not auto-fix it (the `generate-skills.yml` hourly auto-sync workflow was deleted, even though the drift error still tells you an hourly PR will repair it).
