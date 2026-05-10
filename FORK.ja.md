# Fork: MCP サーバー機能を温存した gws

このリポジトリは [googleworkspace/cli](https://github.com/googleworkspace/cli) のフォークです。

[English version](FORK.md)

upstream が削除した **MCP（Model Context Protocol）サーバー機能** を独自にメンテナンスし、AI エージェントから Google Workspace API を直接呼び出せる状態を維持しています。

## upstream との差分

| 項目 | upstream | このフォーク |
|---|---|---|
| MCP サーバー (`gws mcp`) | 削除済み | 維持・メンテナンス中 |
| MCP helper tools (`--helpers`) | なし | `gmail_send` 等を独自実装 |
| HTTP transport (`--transport http`) | なし | Streamable HTTP（Phase 1: 認証なし） |
| OAuth2 PKCE 認証 (`--auth`) | なし | MCP spec 2025-11-25 準拠の AS（RFC 9728 + RFC 8414 + PKCE S256） |
| CI/CD ワークフロー | upstream 環境依存 | 最小構成（CI + Policy + Sync + Release） |

### MCP サーバー

Discovery Document から動的にツールを生成し、stdio 経由で MCP プロトコルを提供します。

```bash
# Gmail の MCP サーバーを起動（helper tool 付き）
gws mcp -s gmail --helpers

# 複数サービスを同時に提供（カンマ区切り）
gws mcp -s gmail,drive,calendar --helpers

# compact モード（サービスごとに1ツール）
gws mcp -s gmail --tool-mode compact
```

### MCP helper tools

`--helpers` フラグで有効化される便利ツールです。Discovery API の raw tool に加え、RFC 2822 構築や base64 エンコード等の面倒な処理を自動化します。

| ツール名 | 説明 |
|---|---|
| `gmail_send` | メール送信。to/subject/body を渡すだけで RFC 2822 フォーマット・base64url エンコードを自動処理 |
| `gmail_reply` | スレッド内返信。message_id/body を渡すだけで In-Reply-To, References, Re: 件名, threadId を自動設定 |

## インストール

### Homebrew（macOS / Linux）— 推奨

```bash
brew install shigechika/tap/gws-mcp
```

Rust ツールチェーン不要。macOS（Apple Silicon / Intel）と Linux（x86\_64 / arm64）向けのバイナリを事前ビルドして配布しています。

### Debian / Ubuntu（.deb）

```bash
sudo dpkg -i gws-mcp-<VERSION>-linux-amd64.deb
# arm64 の場合:
sudo dpkg -i gws-mcp-<VERSION>-linux-arm64.deb
```

`.deb` ファイルは[最新リリース](https://github.com/shigechika/gws-mcp/releases/latest)からダウンロードしてください。

### RHEL / Fedora / Amazon Linux（.rpm）

```bash
sudo rpm -i gws-mcp-<VERSION>-linux-amd64.rpm
# aarch64 の場合:
sudo rpm -i gws-mcp-<VERSION>-linux-arm64.rpm
```

`.rpm` ファイルは[最新リリース](https://github.com/shigechika/gws-mcp/releases/latest)からダウンロードしてください。

### Windows

[最新リリース](https://github.com/shigechika/gws-mcp/releases/latest)から `gws-mcp-<VERSION>-windows-amd64.zip` をダウンロードし、`gws.exe` を展開して `PATH` の通ったディレクトリに配置してください。

### ダイレクトダウンロード（macOS / Linux）

[最新リリース](https://github.com/shigechika/gws-mcp/releases/latest)からプラットフォーム向けの `.tar.gz` をダウンロードし、`gws` を `PATH` の通った場所に配置してください。

| プラットフォーム | アーカイブ |
|---|---|
| macOS（Apple Silicon） | `gws-mcp-<VERSION>-macos-arm64.tar.gz` |
| macOS（Intel） | `gws-mcp-<VERSION>-macos-amd64.tar.gz` |
| Linux x86\_64 | `gws-mcp-<VERSION>-linux-amd64.tar.gz` |
| Linux arm64 | `gws-mcp-<VERSION>-linux-arm64.tar.gz` |

### Cargo（ソースからビルド）

```bash
# GitHub から直接インストール
cargo install --git https://github.com/shigechika/gws-mcp --locked
```

ローカルに clone 済みの場合は、ワーキングツリーからインストール:

```bash
cd gws-mcp
cargo install --path crates/google-workspace-cli
```

`~/.cargo/bin/gws` にバイナリがインストールされます。`cargo build --release` は `target/release/gws` にビルドするだけで `~/.cargo/bin/` は**更新されない**点に注意してください。

## Claude での使い方

**Claude Code** — `~/.claude.json` に追加:

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

**Claude Desktop** — `~/Library/Application Support/Claude/claude_desktop_config.json`（macOS）に追加:

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

### HTTP transport（Streamable HTTP）

サーバーを先に起動します:

```bash
gws mcp -s gmail,drive,calendar --helpers --transport http --port 3000
```

Claude Code からは `command`/`args` 不要で URL だけ指定します:

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

デフォルトのバインドは `127.0.0.1`（ループバックのみ）。`--bind 0.0.0.0` で外部からもアクセス可能になりますが、`--auth` なしでの使用は推奨しません。

> **`--bind` と OAuth2 resource URL について:** ループバックバインド（`127.0.0.1`、`0.0.0.0`、`::`、`::1`）はいずれも RFC 9728 Protected Resource Metadata に `http://localhost:<port>` を広告します。クライアントが接続に使う URL と一致させるためです。非ループバックアドレス（特定 IP やホスト名）はそのまま使用されます。

### OAuth2 PKCE 認証（`--auth`）

[MCP Authorization spec 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization/) に準拠した OAuth2 Authorization Server を HTTP transport 上で有効化します。

**前提条件:**
1. `gws auth setup` で Google OAuth2 ウェブアプリ認証情報 (`client_secret.json`) を作成
2. [Google Cloud Console](https://console.cloud.google.com/apis/credentials) で `http://localhost:<port>/oauth/callback` を **承認済みリダイレクト URI** に追加

```bash
gws mcp -s gmail,drive,calendar --helpers --transport http --port 3000 --auth
```

公開される OAuth2 エンドポイント:

| エンドポイント | RFC | 用途 |
|---|---|---|
| `/.well-known/oauth-protected-resource` | RFC 9728 | Protected Resource Metadata |
| `/.well-known/oauth-authorization-server` | RFC 8414 | Authorization Server Metadata |
| `/oauth/register` | RFC 7591（スタブ） | Dynamic Client Registration |
| `/oauth/authorize` | RFC 6749 | 認可エンドポイント — Google にリダイレクト |
| `/oauth/callback` | — | Google OAuth2 コールバック |
| `/oauth/token` | RFC 6749 | トークンエンドポイント — コード + PKCE verifier をベアラートークンに交換 |

`/mcp` へのリクエストはすべて有効な `Authorization: Bearer <token>` ヘッダが必要です。セッションは8時間で失効します。

認証済みユーザーの MCP ツール呼び出しは、OAuth フロー中に取得したそのユーザー自身の Google アクセストークンを使用します。スコープは認可時のサービス一覧（例: `-s gmail,drive`）から導出されます。`--auth` 無効時は共有の `gws auth login` 認証情報にフォールバックします。

> **現在の制限:**
> - ユーザーごとのトークンは**メモリ上にのみ保存**されます。`gws mcp` を再起動するとトークンが消去され、再認証が必要になります。
> - GWS スコープは [`DEFAULT_SCOPES`](crates/google-workspace-cli/src/auth_commands.rs) のみから導出されます。このリスト外のサービス（例: `admin`、`script`）は認可時に固有のスコープがリクエストされず、API 呼び出しが権限エラーになる場合があります。
> いずれも将来のリリースで対応予定です。

## このフォークで対応した upstream の MCP issue

upstream の MCP サーバーに対するバグ報告・機能要望（MCP 削除に伴い close されたもの）を、このフォークで移植・対応しています。

| upstream issue | 状態 | 内容 |
|---|---|---|
| [#162](https://github.com/googleworkspace/cli/issues/162) — `tools/list` が呼び出せないツール名を返す（alias と doc.name の不一致） | 対応済 | `walk_resources` がツール名プレフィックスに Discovery doc 名ではなく設定された alias を使うよう変更。`tools/list` と `tools/call` の名前空間を統一 |
| [#170](https://github.com/googleworkspace/cli/issues/170) — 複数単語のリソース名（`admin_role_assignments_list` 等）でパースが壊れる | 対応済 | `split('_')` を Discovery ツリーに対する貪欲リゾルバ（`resolve_tool_path`）に置換。アンダースコアを含むリソース名・任意の入れ子に対応 |
| [#212](https://github.com/googleworkspace/cli/issues/212) — Full mode の schema が GET メソッドにも `body`/`upload` を含む | 対応済 | `method.request.is_some()` の時のみ `body` を、`supports_media_upload == true` の時のみ `upload` を付与 |
| [#251](https://github.com/googleworkspace/cli/issues/251) — `--upload` が絶対パス・トラバーサルパスを受理する | 対応済 | MCP の `upload` 引数で絶対パス・`..` 要素を拒否 |
| [#260](https://github.com/googleworkspace/cli/issues/260) — tool annotations（`readOnlyHint` / `destructiveHint` / `idempotentHint`） | 部分対応 | HTTP method から導出した annotations を全ツールに付与。`tool_search` メタツールとページネーションは未移植 |
| [#642](https://github.com/googleworkspace/cli/issues/642) — `parse_message_headers` の case-sensitive マッチが `CC` 等の非正規ケースのヘッダを落とす | 対応済 | ヘッダ名を小文字化してからマッチするよう変更。Exchange/Outlook 由来の `"CC"` 等、RFC 5322 §1.2.2 に沿った任意ケーシングを認識 |
| [#573](https://github.com/googleworkspace/cli/issues/573) — `gmail.users.messages.get` で `metadataHeaders` 配列がクエリパラメータに展開されない | 対応済 | Discovery パーサが `repeated: true` を保持（`discovery.rs`）し、JSON 配列値を複数クエリに展開する実装が入っている（`executor.rs`）。Discovery 駆動の MCP ツールも同じ挙動を継承 |
| [#625](https://github.com/googleworkspace/cli/issues/625) — `script` service が `services.rs` に未登録で helper が到達不能 | 対応済 | `ServiceEntry { aliases: &["script"], api_name: "script", version: "v1", ... }` として登録済み。`gws script ...` と MCP `script_*` ツールが正常に解決する |
| [#717](https://github.com/googleworkspace/cli/issues/717) — `gws auth status` が非 JSON を stdout に出力し `jq` パイプラインを破壊 | 対応済 | `Using keyring backend: <name>` は `credential_store.rs` で `eprintln!`（stderr）に出力される。`gws auth status \| jq .` は正常に動作 |
| [#562](https://github.com/googleworkspace/cli/issues/562) — 対話 TUI が `cloud-platform` スコープを無条件に注入し、Workspace の admin policy で制限される組織では login が失敗する | 対応済 | `run_discovery_scope_picker` の選択後 auto-inject を削除（`auth_commands.rs`）。`cloud-platform` が必要な用途（modelarmor 等）は picker で明示選択するか `--full` / `--scopes` で指定する |
| [#644](https://github.com/googleworkspace/cli/issues/644) — `gmail +send` が `userinfo.profile` スコープを付与済みでも「grant profile scope」ヒントを出し、From の表示名が null になる | 対応済 | `helpers/gmail/mod.rs` の表示名取得を People API (`/people/me?personFields=names`) から OIDC userinfo endpoint (`openidconnect.googleapis.com/v1/userinfo`) に変更。同じスコープで Workspace / 個人 Gmail どちらでも一貫したレスポンスが得られる。401/403 時のフォールバックメッセージも、一時的な拒否をスコープ欠落と誤診断しない表現に改訂 |

## upstream MCP 定点観測

| 時期 | 出来事 |
|---|---|
| 2026-03-04 | `feat: add gws mcp server` — upstream に MCP サーバーが追加 |
| 2026-03-05 | ブランチ `fix/mcp-hyphen-tool-names` が upstream に出現 — ツール名の区切り文字をアンダースコアからハイフンに変更 |
| 2026-03-06 | `fix!: Remove MCP server mode` — 追加からわずか2日で upstream が breaking change として MCP サーバーを削除 |
| 2026-03-06 | 同ブランチがマージされずに削除 — upstream での MCP 復活は見送り |

## upstream 同期方針

- 毎週月曜に GitHub Actions で upstream/main を自動マージ
- コンフリクト発生時は PR を作成して手動解決
- MCP 関連コード（`src/mcp_server.rs`、`pub(crate)` 可視性）の温存を最優先
- upstream のコミットメッセージから `#番号` 参照を除去（クロスリファレンス防止）
