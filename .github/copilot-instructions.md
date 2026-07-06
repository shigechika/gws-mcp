# Repository overview

`gws` is a Rust CLI for Google Workspace APIs. It dynamically generates its
command surface at runtime by parsing Google Discovery Service JSON documents
— it does **not** use generated crates like `google-drive3`. This repository
is `shigechika/gws-mcp`, a **fork of `googleworkspace/cli`** that keeps an
MCP (Model Context Protocol) server upstream removed. It stays close to
upstream on purpose: minimize divergence so future upstream merges stay
low-conflict.

Cargo workspace, two crates:

- `crates/google-workspace/` — publishable library (Discovery models, HTTP
  client, path/URL/resource validators in `validate.rs`)
- `crates/google-workspace-cli/` — the `gws` binary (arg parsing, executor,
  auth, helpers, and the fork-only `mcp_server.rs`)

Full architecture, workspace layout, and helper-command conventions are in
`AGENTS.md` at the repo root — read it before reviewing changes to
`executor.rs`, `validate.rs`, or anything under `helpers/`.

# Build & validate

```bash
cargo build                          # dev build, both crates
cargo test -p google-workspace       # library unit tests
cargo test -p google-workspace-cli   # CLI unit tests
cargo clippy --workspace -- -D warnings   # exact command CI runs — not --all-targets
cargo fmt --check
```

A `.rs`/`Cargo.*` change without a `.changeset/*.md` file fails the `Policy
Check` CI job. Changeset format:

```markdown
---
"@googleworkspace/cli": patch
---

One-paragraph description of the fix.
```

# What to focus review on in this repo

## 1. URL construction is a real attack surface — this CLI is driven by LLM/MCP agents

`AGENTS.md` → "Input Validation & URL Safety" defines the contract precisely.
Check every diff that builds a URL against it:

- **Path segments**: user input must go through
  `crate::validate::encode_path_segment()`. Never format raw input into a
  path.
- **Query parameters**: must go through reqwest's `.query()` builder, NOT
  raw `format!("...?key={value}")` string interpolation. A value containing
  `&` in a hand-formatted query string silently truncates/splits into extra
  parameters; `+` gets decoded server-side as a space. (This exact class of
  bug shipped once in `helpers/modelarmor.rs` and was only caught in manual
  review, not by an earlier automated pass — flag any `format!` containing
  `?` or `&` next to a variable.)
- **Resource names** (project IDs, space names, topic names) must go through
  `crate::validate::validate_resource_name()` before being embedded anywhere.
- If a diff touches `crate::validate::encode_path_segment`,
  `validate_resource_name`, or any other function in `validate.rs`: grep for
  **every** caller (there are ~7 across `executor.rs`, `helpers/gmail/`,
  `helpers/calendar.rs`, `helpers/workflows.rs`, `helpers/modelarmor.rs`) and
  check each one still gets the behavior it relies on — these are shared,
  security-sensitive utilities, not single-use helpers. A change that's
  correct for the file it's in can still break a different caller with a
  different usage pattern (e.g. a path-segment encoder reused for a query
  value).

## 2. Fork-vs-upstream discipline

- Prefer the **smallest possible diff** that fixes the issue, especially in
  files that exist upstream (not `mcp_server.rs`, which is fork-only). A
  fix that duplicates upstream's own in-flight PR almost byte-for-byte is
  preferred over a "better" but divergent rewrite — divergence is a future
  merge-conflict cost, not just a style choice. If a PR description says
  "backports upstream #NNNN," confirm the diff is actually a narrow,
  faithful match to that upstream PR before suggesting unrelated
  refactoring in the same hunk.
- MCP helper tools follow a **façade pattern** (see `AGENTS.md` → "MCP
  Helper Tools"): never flag `pub(super)` → `pub(crate)` changes on
  upstream-owned functions as acceptable — the correct fix is a `pub(crate)`
  façade appended at the end of the file in a `// Fork-only: MCP bridge
  functions` block, not loosening the original visibility.
- New MCP helper tool handlers in `mcp_server.rs` must wrap their return
  value in the MCP content envelope (`{"content": [{"type": "text", "text":
  ...}], "isError": false}`). A handler that returns a bare JSON object
  compiles and passes unit tests but produces "completed with no output" in
  a real MCP client — flag any new `handle_*` function in `mcp_server.rs`
  that doesn't route through the existing `json_text_content()` helper (or
  equivalent) before returning.

## 3. Test the failure path, not just the happy path

Per `AGENTS.md`'s checklist: new validation logic needs a test asserting
`Err` for the rejected case (e.g. `../../.ssh`), not just `Ok` for valid
input. When a diff changes what characters an encoder/validator
accepts-vs-rejects, check whether existing tests only assert on inputs that
happen to not exercise the changed boundary (e.g. a test ID with no `-`/`.`/
`_`/`&` in it can't catch a regression in how those characters are handled).

## 4. Auth, credentials, and multi-profile config

This CLI is often run non-interactively (by an MCP client or a script), so
silent auth failures are worse than loud ones. When a diff touches `auth.rs`,
`auth_commands.rs`, `credential_store.rs`, or `token_storage.rs`:

- Keep these three concepts distinct and don't let a diff blur them:
  `client_secret.json` (OAuth app config, no refresh token, read only by
  `auth login`), `GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE` (an exported
  credential *with* a refresh token, read only at API-call time — never by
  `auth login`), and `GOOGLE_WORKSPACE_CLI_CONFIG_DIR` (switches the whole
  profile: client secret + encrypted credentials + token cache together).
- The AES key protecting `credentials.enc` can live in the OS keyring or in
  a `.encryption_key` file (`GOOGLE_WORKSPACE_CLI_KEYRING_BACKEND=file`,
  used for headless/CI/Docker contexts like the MCP server). A diff that
  changes how credentials are read/written must keep `auth login` and every
  consumer using the **same** backend — a mismatch fails closed by silently
  falling back to bare ADC (losing whatever scopes were actually granted),
  not with a clear error.
- Token cache keys must be namespaced per account/credential identity, not
  just per OAuth scope set — two different accounts requesting the same
  scopes must not collide on the same cache entry.

## 5. GitHub Actions / workflow file changes

- `GITHUB_TOKEN` cannot create or update any file under `.github/workflows/*`
  — there is no grantable `workflows` permission scope, so this can't be
  fixed via the `permissions:` block. Any workflow that might push a commit
  touching `.github/workflows/*` (e.g. an upstream-sync job) needs a PAT
  with `workflow` scope for that push, not just broader `contents`/
  `pull-requests` permissions.
- If a secret-backed token (e.g. `${{ secrets.SOME_PAT || secrets.GITHUB_TOKEN
  }}`) is repeated across more than one step, prefer a job-level `env:`
  entry over copy-pasting the same fallback expression — a future edit that
  updates one copy and misses another silently reintroduces whatever bug
  the token was added to fix.
- `actions/checkout`'s `token:` input persists credentials **host-wide**
  (`http.https://github.com/.extraheader`), not scoped to just that job's
  `origin` remote — if the job also talks to a different repo (e.g.
  fetching from upstream), that credential is sent there too. This is
  usually harmless for a token with at least public-repo read access, but
  call it out if the token is meant to be tightly scoped.

## 6. Bilingual docs must move together

Where a `.ja.md` counterpart exists (`README.md`/`README.ja.md`,
`FORK.md`/`FORK.ja.md`), a diff that updates one should update the other in
the same PR, not as a follow-up. Flag a PR that changes only the English or
only the Japanese side of a pair when the change isn't purely
language-specific (e.g. a new CLI flag, a new table row, a new section).

# Out of scope for review comments

- Formatting/style nits `cargo fmt`/`clippy` already enforce in CI.
- Suggesting `.gitignore`d or upstream-owned workflow files be modified
  directly — see `sync-upstream.yml` for how upstream merges are handled.
