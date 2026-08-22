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

### Docker

```bash
docker run -i --rm \
  -v ~/.config/gws:/home/gws/.config/gws \
  ghcr.io/shigechika/gws-mcp:latest
```

認証情報はホスト側に保存し、コンテナにマウントします。初回のみセットアップを実行してください:

```bash
# 初回セットアップ（ホスト上、または docker run -it で実行）
docker run -it --rm -v ~/.config/gws:/home/gws/.config/gws ghcr.io/shigechika/gws-mcp auth setup
docker run -it --rm -v ~/.config/gws:/home/gws/.config/gws ghcr.io/shigechika/gws-mcp auth login
```

利用可能なタグ: `latest`、`<VERSION>`（例: `0.22.5-mcp.1`）

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

### MCP Registry（Docker）

gws-mcp は [Official MCP Registry](https://registry.modelcontextprotocol.io/) に `io.github.shigechika/gws-mcp` として登録されています。Registry に対応した MCP クライアントは自動的に検出・設定できます。

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

### リバースプロキシ経由での公開（`--public-url`）

Caddy や nginx で TLS 終端を行うリバースプロキシ経由で `gws mcp` を公開する場合は、`--public-url` で RFC 9728 / RFC 8414 の OAuth2 メタデータに広告するベース URL を上書きします:

```bash
gws mcp -s gmail,drive,calendar --helpers \
    --transport http --port 3000 --bind 127.0.0.1 --auth \
    --public-url https://mcp.example.com/gws
```

Caddy 設定例（`/gws` プレフィックスを除去して転送）:

```
mcp.example.com {
    handle_path /gws/* {
        reverse_proxy localhost:3000
    }
}
```

[Google Cloud Console](https://console.cloud.google.com/apis/credentials) で `https://mcp.example.com/gws/oauth/callback` を**承認済みリダイレクト URI** に追加してください。

`--public-url` には `/mcp` や `/oauth` パスを含めないでください — これらのパスは自動的に付加されます。末尾のスラッシュは無視されます。

## 認証とプロファイル（混同注意）

`gws` の認証まわりで紛らわしい3つを整理します。特に複数アカウント/プロファイルを切り替えるときに混同しやすいので注意してください。

| 名前 | 中身 | 使われる場面 |
|---|---|---|
| `client_secret.json` | OAuth クライアントアプリ設定（client_id / client_secret、`refresh_token` は**無い**） | `gws auth login` が OAuth フローを回すために読む。場所は `<config_dir>/client_secret.json` |
| `GOOGLE_WORKSPACE_CLI_CONFIG_DIR` | 設定ディレクトリ全体（client_secret.json・credentials.enc・token_cache.json を内包） | プロファイルごと丸ごと切り替える |
| `GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE` | 取得済み資格情報（`refresh_token` 入りの authorized_user、または SA キー＝`gws auth export` の出力） | API 呼び出し時にトークン化に直接使う。**`auth login` は読まない** |

```bash
# ✅ work プロファイルでログイン＆運用（login と後続コマンド両方に付ける）
GOOGLE_WORKSPACE_CLI_CONFIG_DIR=~/.config/gws-work gws auth login
GOOGLE_WORKSPACE_CLI_CONFIG_DIR=~/.config/gws-work gws gmail users getProfile --params '{"userId":"me"}'

# ✅ CREDENTIALS_FILE を使うのは「export 済み資格情報」を渡すときだけ（refresh_token 入り）
gws auth export --unmasked 2>/dev/null > /tmp/work.json
GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE=/tmp/work.json gws gmail users getProfile --params '{"userId":"me"}'

# ❌ やらない: client_secret.json を CREDENTIALS_FILE に渡す
#    → 型違い（refresh_token なし）。auth login はこの env を無視し、
#      さらに同じシェルの後続 API 呼び出しが壊れる
GOOGLE_WORKSPACE_CLI_CREDENTIALS_FILE=~/.config/gws-work/client_secret.json gws auth login
```

覚え方:
- **`CONFIG_DIR`** = フォルダごと切り替え（プロファイル分離はこれ）
- **`CREDENTIALS_FILE`** = 鍵そのもの（`gws auth export` の出力を渡す）
- **`client_secret.json`** = アプリ設定（`auth login` 専用）

> **keyring backend も揃える。** `credentials.enc` を暗号化する AES 鍵は、OS キーリング（既定）か、ローカルの `.encryption_key` ファイル（`GOOGLE_WORKSPACE_CLI_KEYRING_BACKEND=file`。MCP サーバーなどヘッドレス用途で推奨）に保存されます。**`auth login` と、その認証情報を使う側（MCP サーバー等）は同じ backend を使う必要があります。** 食い違うと、使う側が `credentials.enc` を復号できず「壊れている」と判断して削除し、黙って ADC（`GOOGLE_APPLICATION_CREDENTIALS`）にフォールバックします（たいてい `insufficient authentication scopes` として表面化）。MCP サーバーが `KEYRING_BACKEND=file` で動くなら、ログインも同じく付けます:
>
> ```bash
> GOOGLE_WORKSPACE_CLI_CONFIG_DIR=~/.config/gws-work GOOGLE_WORKSPACE_CLI_KEYRING_BACKEND=file gws auth login
> ```

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
| [#556](https://github.com/googleworkspace/cli/issues/556) — `gws auth login` が People/Meet の OAuth スコープを一切提示しない | 対応済（HTTP transport） | 対話 CLI の scope picker は Discovery ドキュメント経由で動的にスコープを発見するため問題なし。フォークの `--auth` HTTP transport はスコープを `gws_scopes_for_services` で静的導出しており、`people`/`meet` のマッピング（`map_service_to_scope_prefixes`）自体は存在するのに、照合対象となる候補スコープが1件も無かった。この関数専用の候補セット `HTTP_TRANSPORT_EXTRA_SCOPES`（`contacts.readonly`, `meetings.space.created`）を追加し、`MINIMAL_SCOPES`/`DEFAULT_SCOPES`（＝CLI 自体のデフォルト login）には影響を与えないようにした |
| [#644](https://github.com/googleworkspace/cli/issues/644) — `gmail +send` が `userinfo.profile` スコープを付与済みでも「grant profile scope」ヒントを出し、From の表示名が null になる | 対応済 | `helpers/gmail/mod.rs` の表示名取得を People API (`/people/me?personFields=names`) から OIDC userinfo endpoint (`openidconnect.googleapis.com/v1/userinfo`) に変更。同じスコープで Workspace / 個人 Gmail どちらでも一貫したレスポンスが得られる。401/403 時のフォールバックメッセージも、一時的な拒否をスコープ欠落と誤診断しない表現に改訂 |
| [#886](https://github.com/googleworkspace/cli/issues/886) — 復号失敗時に credentials file がエラー説明なしでサイレント削除される | 対応済 | `credentials.enc` の復号に失敗した場合（keyring/暗号化キーの変更後等）、削除ではなく `credentials.enc.unreadable.<timestamp>` へのリネームに変更し、実際の復号エラーとリネーム結果を表示するようにした。タイムスタンプを付与することで、後発の失敗が先発の失敗で保存したファイルを上書きしてしまうことを防ぐ。トークンキャッシュ（`token_cache.json`, `sa_token_cache.json`）は再ログインで再導出可能なため引き続き削除する |

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
