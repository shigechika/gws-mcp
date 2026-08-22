# Fork: gws with MCP server support

This repository is a fork of [googleworkspace/cli](https://github.com/googleworkspace/cli).

It maintains the **MCP (Model Context Protocol) server** that upstream removed, allowing AI agents to call Google Workspace APIs directly.

[日本語版はこちら](FORK.ja.md)

## Differences from upstream

| Feature | upstream | This fork |
|---|---|---|
| MCP server (`gws mcp`) | Removed | Maintained |
| MCP helper tools (`--helpers`) | N/A | `gmail_send` and more |
| HTTP transport (`--transport http`) | N/A | Streamable HTTP (Phase 1: no auth) |
| OAuth2 PKCE auth (`--auth`) | N/A | MCP spec 2025-11-25 compliant AS (RFC 9728 + RFC 8414 + PKCE S256) |
| CI/CD workflows | Upstream-specific | Minimal (CI + Policy + Sync + Release) |

### MCP server

Dynamically generates tools from Discovery Documents and serves them via the MCP protocol over stdio.

```bash
# Start MCP server for Gmail with helper tools
gws mcp -s gmail --helpers

# Serve multiple services (comma-separated)
gws mcp -s gmail,drive,calendar --helpers

# Compact mode (one tool per service)
gws mcp -s gmail --tool-mode compact
```

### MCP helper tools

Enabled with the `--helpers` flag. These provide high-level operations on top of the raw Discovery API tools, automating tedious tasks like RFC 2822 formatting and base64url encoding.

| Tool | Description |
|---|---|
| `gmail_send` | Send email. Just pass to/subject/body — RFC 2822 formatting and base64url encoding are handled automatically |
| `gmail_reply` | Reply within a thread. Pass message_id/body — In-Reply-To, References, Re: subject, and threadId are set automatically |
| `gmail_read` | Read a message. Pass message_id — returns parsed headers and the decoded plain-text body as compact JSON, skipping the raw MIME/base64 payload |

## Installation

### Homebrew (macOS / Linux) — recommended

```bash
brew install shigechika/tap/gws-mcp
```

No Rust toolchain required. Binaries are pre-built for macOS (Apple Silicon / Intel) and Linux (x86\_64 / arm64).

### Debian / Ubuntu (.deb)

```bash
sudo dpkg -i gws-mcp-<VERSION>-linux-amd64.deb
# or for arm64:
sudo dpkg -i gws-mcp-<VERSION>-linux-arm64.deb
```

Download the `.deb` file from the [latest release](https://github.com/shigechika/gws-mcp/releases/latest).

### RHEL / Fedora / Amazon Linux (.rpm)

```bash
sudo rpm -i gws-mcp-<VERSION>-linux-amd64.rpm
# or for aarch64:
sudo rpm -i gws-mcp-<VERSION>-linux-arm64.rpm
```

Download the `.rpm` file from the [latest release](https://github.com/shigechika/gws-mcp/releases/latest).

### Windows

Download `gws-mcp-<VERSION>-windows-amd64.zip` from the [latest release](https://github.com/shigechika/gws-mcp/releases/latest), extract `gws.exe`, and place it in a directory on your `PATH`.

### Direct download (macOS / Linux)

Download the `.tar.gz` archive for your platform from the [latest release](https://github.com/shigechika/gws-mcp/releases/latest) and place `gws` in your `PATH`.

| Platform | Archive |
|---|---|
| macOS (Apple Silicon) | `gws-mcp-<VERSION>-macos-arm64.tar.gz` |
| macOS (Intel) | `gws-mcp-<VERSION>-macos-amd64.tar.gz` |
| Linux x86\_64 | `gws-mcp-<VERSION>-linux-amd64.tar.gz` |
| Linux arm64 | `gws-mcp-<VERSION>-linux-arm64.tar.gz` |

### Docker

```bash
docker run -i --rm \
  -v ~/.config/gws:/home/gws/.config/gws \
  ghcr.io/shigechika/gws-mcp:latest
```

Credentials are stored on the host and mounted into the container. Run the setup commands once:

```bash
# First-time setup (run on host or via docker run -it)
docker run -it --rm -v ~/.config/gws:/home/gws/.config/gws ghcr.io/shigechika/gws-mcp auth setup
docker run -it --rm -v ~/.config/gws:/home/gws/.config/gws ghcr.io/shigechika/gws-mcp auth login
```

Available tags: `latest`, `<VERSION>` (e.g. `0.22.5-mcp.1`).

### Cargo (from source)

```bash
# Install directly from GitHub
cargo install --git https://github.com/shigechika/gws-mcp --locked
```

If you cloned the repository locally:

```bash
cd gws-mcp
cargo install --path crates/google-workspace-cli
```

This installs the binary to `~/.cargo/bin/gws`. Note that `cargo build --release` only builds to `target/release/gws` and does **not** update `~/.cargo/bin/`.

## Usage with Claude

**Claude Code** — add to `~/.claude.json`:

```json
{
  "mcpServers": {
    "gws": {
      "command": "gws",
      "args": ["mcp", "-s", "gmail,drive,calendar", "--helpers"]
    }
  }
}
```

**Claude Desktop** — add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "gws": {
      "command": "gws",
      "args": ["mcp", "-s", "gmail,drive,calendar", "--helpers"]
    }
  }
}
```

### MCP Registry (Docker)

gws-mcp is listed in the [Official MCP Registry](https://registry.modelcontextprotocol.io/) as `io.github.shigechika/gws-mcp`. MCP clients that support the registry can discover and configure it automatically.

### HTTP transport (Streamable HTTP)

Start the server first:

```bash
gws mcp -s gmail,drive,calendar --helpers --transport http --port 3000
```

Then point Claude Code at it — no `command`/`args` needed, just a URL:

```json
{
  "mcpServers": {
    "gws": {
      "type": "http",
      "url": "http://localhost:3000/mcp"
    }
  }
}
```

The server binds to `127.0.0.1` by default (loopback only). Use `--bind 0.0.0.0` to allow external access (not recommended without additional auth).

> **Note on `--bind` and OAuth2 resource URLs:** Loopback binds (`127.0.0.1`, `0.0.0.0`, `::`, `::1`) all advertise `http://localhost:<port>` in the RFC 9728 Protected Resource Metadata. This ensures the `resource` URL matches what clients connect to. Non-loopback bind addresses (e.g. a specific IP or hostname) are used as-is.

### OAuth2 PKCE authentication (`--auth`)

Enables a full OAuth2 Authorization Server on the HTTP transport, compliant with the [MCP Authorization spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization/).

**Prerequisites:**
1. Run `gws auth setup` to create `client_secret.json` with a Google OAuth2 web app credential
2. Add `http://localhost:<port>/oauth/callback` as an **Authorized redirect URI** in [Google Cloud Console](https://console.cloud.google.com/apis/credentials)

```bash
gws mcp -s gmail,drive,calendar --helpers --transport http --port 3000 --auth
```

The server exposes these OAuth2 endpoints:

| Endpoint | RFC | Purpose |
|---|---|---|
| `/.well-known/oauth-protected-resource` | RFC 9728 | Protected Resource Metadata |
| `/.well-known/oauth-authorization-server` | RFC 8414 | Authorization Server Metadata |
| `/oauth/register` | RFC 7591 (stub) | Dynamic Client Registration |
| `/oauth/authorize` | RFC 6749 | Authorization endpoint — redirects to Google |
| `/oauth/callback` | — | Google OAuth2 callback |
| `/oauth/token` | RFC 6749 | Token endpoint — exchanges code + PKCE verifier for bearer token |

All requests to `/mcp` require a valid `Authorization: Bearer <token>` header. Sessions expire after 8 hours.

Each authenticated user's MCP tool calls use their own Google access token obtained during the OAuth flow — GWS scopes are derived from the configured service list (e.g. `-s gmail,drive`) at authorize time. The shared `gws auth login` credential is used only as a fallback when `--auth` is disabled.

> **Current limitations:**
> - Per-user tokens are stored **in memory only**. Restarting `gws mcp` clears all tokens and requires users to re-authenticate.
> - GWS scopes are derived from [`DEFAULT_SCOPES`](crates/google-workspace-cli/src/auth_commands.rs) only. Services whose scopes fall outside that static list (e.g. `admin`, `script`) will not have their specific scopes requested at authorize time; API calls for those services may fail with permission errors.
> Both limitations are planned for a future release.

### Reverse proxy deployment (`--public-url`)

When `gws mcp` runs behind a reverse proxy (e.g. Caddy, nginx) with TLS termination, use `--public-url` to override the base URL advertised in RFC 9728 / RFC 8414 OAuth2 metadata:

```bash
gws mcp -s gmail,drive,calendar --helpers \
    --transport http --port 3000 --bind 127.0.0.1 --auth \
    --public-url https://mcp.example.com/gws
```

Example Caddy configuration (strips `/gws` prefix before forwarding):

```
mcp.example.com {
    handle_path /gws/* {
        reverse_proxy localhost:3000
    }
}
```

Register `https://mcp.example.com/gws/oauth/callback` as an **Authorized redirect URI** in [Google Cloud Console](https://console.cloud.google.com/apis/credentials).

The `--public-url` value must not include `/mcp` or `/oauth` paths — those are appended automatically. Trailing slashes are ignored.

## Authentication and profiles (easily confused)

Three auth concepts in `gws` are easy to mix up — especially when switching between multiple accounts/profiles:

| Name | What it is | When it's used |
|---|---|---|
| `client_secret.json` | OAuth client-app config (client_id / client_secret; **no** `refresh_token`) | Read by `gws auth login` to run the OAuth flow. Located at `<config_dir>/client_secret.json` |
| `GOOGLE_WORKSPACE_CLI_CONFIG_DIR` | The whole config directory (holds client_secret.json + credentials.enc + token_cache.json) | Switch an entire profile at once |
| `GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE` | An already-obtained credential (a `refresh_token`-bearing authorized_user, or a service-account key — i.e. the output of `gws auth export`) | Used directly to mint tokens at API-call time. **`auth login` ignores it** |

```bash
# ✅ Log in and run under a "work" profile (pass it to login AND subsequent commands)
GOOGLE_WORKSPACE_CLI_CONFIG_DIR=~/.config/gws-work gws auth login
GOOGLE_WORKSPACE_CLI_CONFIG_DIR=~/.config/gws-work gws gmail users getProfile --params '{"userId":"me"}'

# ✅ Use CREDENTIALS_FILE only to pass already-exported credentials (refresh_token-bearing)
gws auth export --unmasked 2>/dev/null > /tmp/work.json
GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE=/tmp/work.json gws gmail users getProfile --params '{"userId":"me"}'

# ❌ Don't: pass a client_secret.json to CREDENTIALS_FILE
#    → wrong type (no refresh_token); auth login ignores this env, and it breaks
#      subsequent API calls in the same shell
GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE=~/.config/gws-work/client_secret.json gws auth login
```

Rule of thumb:
- **`CONFIG_DIR`** = switch the whole folder (use this for profile separation)
- **`CREDENTIALS_FILE`** = the key itself (pass the output of `gws auth export`)
- **`client_secret.json`** = app config (for `auth login` only)

> **Keep the keyring backend consistent too.** The AES key that encrypts `credentials.enc` is stored either in the OS keyring (default) or in a local `.encryption_key` file (`GOOGLE_WORKSPACE_CLI_KEYRING_BACKEND=file` — recommended for headless consumers like the MCP server). **`auth login` and whatever consumes the credentials must use the same backend.** If they differ, the consumer can't decrypt `credentials.enc`, deletes it as corrupt, and silently falls back to ADC (`GOOGLE_APPLICATION_CREDENTIALS`) — which usually surfaces as `insufficient authentication scopes`. So if your MCP server runs with `KEYRING_BACKEND=file`, log in the same way:
>
> ```bash
> GOOGLE_WORKSPACE_CLI_CONFIG_DIR=~/.config/gws-work GOOGLE_WORKSPACE_CLI_KEYRING_BACKEND=file gws auth login
> ```

## Upstream MCP issues addressed in this fork

Bug reports and feature requests that targeted upstream's MCP server (closed when MCP was removed). This fork ports the fixes so they remain useful:

| Upstream issue | Status | Notes |
|---|---|---|
| [#162](https://github.com/googleworkspace/cli/issues/162) — `tools/list` returns uncallable tool names for aliased services | Fixed | `walk_resources` now uses the configured service alias as tool-name prefix (instead of Discovery doc name), so `tools/list` and `tools/call` share one namespace |
| [#170](https://github.com/googleworkspace/cli/issues/170) — Tool name parsing breaks on multi-word resources (`admin_role_assignments_list` etc.) | Fixed | Replaced `split('_')` with a greedy Discovery-tree resolver (`resolve_tool_path`). Handles arbitrarily nested resources whose names contain underscores |
| [#212](https://github.com/googleworkspace/cli/issues/212) — Full-mode schemas expose `body`/`upload` on GET-only methods | Fixed | `body` is added only when `method.request.is_some()`; `upload` only when `supports_media_upload` is true |
| [#251](https://github.com/googleworkspace/cli/issues/251) — Dynamic `--upload` accepts unsafe absolute/traversal paths | Fixed | MCP `upload` argument rejects absolute paths and `..` components |
| [#260](https://github.com/googleworkspace/cli/issues/260) — Tool annotations (`readOnlyHint`, `destructiveHint`, `idempotentHint`) | Partial | Annotations derived from HTTP method are now attached to every tool. `tool_search` meta-tool and pagination from the original proposal are not yet ported |
| [#642](https://github.com/googleworkspace/cli/issues/642) — `parse_message_headers` case-sensitive match drops CC/headers with non-canonical casing | Fixed | Normalized header names to lowercase before matching, so `"CC"` from Exchange/Outlook, `"from"` lowercase, etc. are all recognized per RFC 5322 §1.2.2 |
| [#573](https://github.com/googleworkspace/cli/issues/573) — `metadataHeaders` array not expanded as repeated query params in `gmail.users.messages.get` | Fixed | Discovery parser preserves `repeated: true` (`discovery.rs`), and the executor expands JSON array values into multiple query entries (`executor.rs`). Discovery-driven MCP tools inherit the same behavior |
| [#625](https://github.com/googleworkspace/cli/issues/625) — `script` service not registered in `services.rs` (helper unreachable) | Fixed | `ServiceEntry { aliases: &["script"], api_name: "script", version: "v1", ... }` is registered, so `gws script ...` and MCP `script_*` tools resolve correctly |
| [#717](https://github.com/googleworkspace/cli/issues/717) — `gws auth status` prints non-JSON to stdout, breaking `jq` pipelines | Fixed | `Using keyring backend: <name>` is emitted via `eprintln!` to stderr (`credential_store.rs`), so `gws auth status \| jq .` parses cleanly |
| [#562](https://github.com/googleworkspace/cli/issues/562) — Interactive TUI unconditionally injects `cloud-platform` scope, breaking org-restricted accounts | Fixed | Removed the post-selection auto-inject in `run_discovery_scope_picker` (`auth_commands.rs`). Users who need `cloud-platform` (e.g. for modelarmor) can tick it in the picker or pass `--full` / explicit `--scopes` |
| [#556](https://github.com/googleworkspace/cli/issues/556) — `gws auth login` never offers People/Meet OAuth scopes | Fixed (HTTP transport) | The interactive CLI scope picker already discovers scopes dynamically via Discovery documents. The fork's `--auth` HTTP transport derives scopes statically via `gws_scopes_for_services`, which had a mapping (`map_service_to_scope_prefixes`) for `people`/`meet` but no candidate scopes to match against. Added `HTTP_TRANSPORT_EXTRA_SCOPES` (`contacts.readonly`, `meetings.space.created`) as a candidate set used only by that function, so `MINIMAL_SCOPES`/`DEFAULT_SCOPES` — and the CLI's own default login — stay unchanged |
| [#644](https://github.com/googleworkspace/cli/issues/644) — `gmail +send` prints "grant profile scope" tip and sends with null From name even when `userinfo.profile` is granted | Fixed | Switched display-name lookup in `helpers/gmail/mod.rs` from People API (`/people/me?personFields=names`) to the OIDC userinfo endpoint (`openidconnect.googleapis.com/v1/userinfo`), which accepts the same scope and responds consistently across Workspace and personal Gmail accounts. Reworded the 401/403 fallback so it doesn't misdiagnose a transient permission denial as a missing scope |
| [#886](https://github.com/googleworkspace/cli/issues/886) — Credentials file is silently deleted when decryption fails, with no error explaining why | Fixed | When `credentials.enc` fails to decrypt (e.g. after a keyring/encryption-key change), it is now renamed to `credentials.enc.unreadable.<timestamp>` instead of deleted, and the actual decrypt error plus the rename outcome are printed. The timestamp suffix prevents a later failure from silently overwriting an earlier failure's preserved file. The token cache files (`token_cache.json`, `sa_token_cache.json`) are still deleted outright, since they're re-derivable from a fresh login |
| [#769](https://github.com/googleworkspace/cli/issues/769) — `+reply` (plain text) corrupts quoted parent when hard-wrapped lines push prefixed lines past 76 chars | Fixed | Pre-wrap quoted body lines to 73 chars before adding `> ` prefix so no line exceeds 75 chars after prefixing, preventing quoted-printable soft-wrap from injecting spurious `> ` mid-sentence (`helpers/gmail/reply.rs`) |
| [#774](https://github.com/googleworkspace/cli/issues/774) — Gmail attachments get: unpadded base64url data breaks standard decoders | Fixed | Switched body/attachment decoders to `URL_SAFE_NO_PAD` with `.trim_end_matches('=')` so both padded and unpadded base64url input are accepted (`helpers/gmail/mod.rs`) |
| [#438](https://github.com/googleworkspace/cli/issues/438) — No helper to extract a Gmail message body as plain text (agents must parse the raw 10–60 KB API payload) | Fixed | The `gmail +read` CLI helper extracts the body; now also exposed as the `gmail_read` MCP helper tool so agents get parsed headers + decoded body as compact JSON instead of the raw MIME payload (`mcp_server.rs`, `helpers/gmail/mod.rs`) |
| [#572](https://github.com/googleworkspace/cli/issues/572) — Token cache ignores `GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE` when switching accounts (returns the previous account's token for the same scopes) | Fixed | `EncryptedTokenStorage` namespaces each cache entry by a non-reversible account fingerprint (short SHA-256 over refresh token / service-account identity) in addition to scopes, so tokens for different accounts no longer collide (`token_storage.rs`, `auth.rs`) |

## Upstream MCP timeline

| Date | Event |
|---|---|
| 2026-03-04 | `feat: add gws mcp server` — MCP server added to upstream |
| 2026-03-05 | Branch `fix/mcp-hyphen-tool-names` appeared in upstream — tool name separator change from underscore to hyphen |
| 2026-03-06 | `fix!: Remove MCP server mode` — MCP server removed from upstream as a breaking change, just 2 days after introduction |
| 2026-03-06 | Branch `fix/mcp-hyphen-tool-names` deleted without being merged — MCP remains absent from upstream |

## Upstream sync policy

- Weekly auto-merge from upstream/main via GitHub Actions (every Monday)
- Conflicts trigger a PR for manual resolution
- MCP-related code (`src/mcp_server.rs`, `pub(crate)` visibility, MCP bridge façade functions) is preserved as top priority
- Issue/PR number references (`#123`) are stripped from upstream commit messages to prevent cross-references
