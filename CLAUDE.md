# CLAUDE.md - Aether Project Guide

## Project Overview

**Aether (ajj)** は "Infrastructure as Workspace" (IaW) を実現する Jujutsu (jj) VCS のラッパーCLIツールです。AIエージェントによる並列開発を可能にするため、ワークスペース（コードブランチ）とインフラ（コンテナ環境）のライフサイクルを同期させます。

## Tech Stack

- **言語**: Rust 2021 Edition
- **バイナリ名**: `ajj`
- **VCS**: Jujutsu (jj) をラップ
- **バックエンド**: Docker (bollard crate)
- **非同期ランタイム**: tokio
- **CLI**: clap v4

## Project Structure

```
IaW/
├── src/
│   ├── main.rs              # エントリーポイント
│   ├── lib.rs               # ライブラリエクスポート
│   ├── error.rs             # エラー型定義
│   ├── cli/                 # コマンドライン処理
│   │   ├── commands.rs      # clap 定義
│   │   ├── workspace.rs     # workspace add/forget
│   │   ├── run.rs           # run コマンド
│   │   └── status.rs        # status コマンド
│   ├── config/              # 設定管理
│   │   ├── schema.rs        # AetherConfig 構造体
│   │   └── loader.rs        # TOML 読み込み
│   ├── jj/                  # Jujutsu 連携
│   │   ├── delegation.rs    # サブプロセス実行
│   │   └── parser.rs        # 出力パース
│   ├── provisioner/         # リソース管理
│   │   ├── state.rs         # ワークスペース状態
│   │   ├── port_allocator.rs # ポート割り当て
│   │   └── context_injector.rs # テンプレート注入
│   ├── backend/             # コンテナバックエンド
│   │   ├── traits.rs        # Backend trait
│   │   └── docker.rs        # Docker 実装
│   └── output/              # 出力フォーマッタ
│       ├── json.rs          # JSON 出力
│       └── human.rs         # 人間可読出力
├── tests/
│   └── integration_tests.rs
├── Cargo.toml
├── Dockerfile               # Alpine musl ビルド
├── docker-compose.yml
└── aether.toml              # サンプル設定
```

## CLI Commands

```bash
ajj workspace add <destination>   # ワークスペース作成 + コンテナ起動
ajj workspace forget <workspace>  # ワークスペース削除 + コンテナ停止
ajj run -- <command>              # 環境変数をロードしてコマンド実行
ajj status [--json]               # ステータス表示
ajj list [--json]                 # 全ワークスペース一覧
ajj cleanup [--force]             # 孤立コンテナの削除
ajj <jj-command>                  # jj へのパススルー
```

## Configuration (aether.toml)

```toml
[backend]
type = "docker"

[services.postgres]
image = "postgres:15"
ports = ["5432"]
env = { POSTGRES_PASSWORD = "password" }

[injection]
file = ".env"
template = "DATABASE_URL=postgres://postgres:password@localhost:{{ services.postgres.ports.5432 }}/mydb"
```

## Development

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test
```

### Lint

```bash
cargo clippy -- -D warnings
cargo fmt --check
```

### Docker

```bash
docker build -t aether:latest .
docker run --rm aether:latest --help
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| clap | CLI 引数パース |
| tokio | 非同期ランタイム |
| bollard | Docker API |
| handlebars | テンプレートエンジン |
| serde | シリアライズ/デシリアライズ |
| thiserror | エラー定義 |
| fs2 | ファイルロック |
| chrono | 日時処理 |

## Architecture Notes

### Backend Trait

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    async fn provision(&self, namespace: &str, services: &HashMap<String, ServiceSpec>) -> Result<Vec<ResourceHandle>>;
    async fn deprovision(&self, namespace: &str) -> Result<()>;
    async fn status(&self, namespace: &str) -> Result<Vec<ResourceStatus>>;
    fn backend_type(&self) -> &'static str;
}
```

### State Management

- 状態は `.jj/aether-state.json` に保存
- ファイルロック (`fs2`) で排他制御
- アトミック書き込み (tmp → rename)

### Port Allocation

- OS に空きポートを割り当てさせる (`TcpListener::bind("127.0.0.1:0")`)
- `Mutex<HashSet<u16>>` でスレッドセーフに管理

## Testing

テストは `#[test]` と `#[tokio::test]` を使用:

```bash
# 全テスト実行
cargo test

# 特定のテストを実行
cargo test test_port_allocator

# Docker 統合テスト (Docker 必要)
cargo test test_docker -- --ignored
```

## CI/CD

GitHub Actions ワークフロー:
- `.github/workflows/ci.yml` - CI (check, fmt, clippy, test, build)
- `.github/workflows/release.yml` - リリース自動化
