# Aether (ajj)

**Infrastructure as Workspace** - AI並列開発のためのミッシングリンク

Aether は [Jujutsu (jj)](https://github.com/martinvonz/jj) のラッパーで、**インフラのライフサイクル**をバージョン管理と統合します。

Claude Code や GitHub Copilot Workspace などの AI エージェント時代に向けて設計されており、ポート競合やリソース枯渇なしに、数十の並列開発環境を即座に立ち上げられます。

## コンセプト

**従来の開発：**
- ブランチは軽量で独立している
- しかし環境（DB、サーバー等）は重く、共有され、競合しがち

**Aether による解決：**
- `ajj workspace add` でブランチと専用コンテナ環境を同時に作成
- インフラは使い捨て：ワークスペースと運命を共にする

## 特徴

- **AI ファースト設計**: 構造化 JSON 出力と決定論的な環境構築で LLM エージェントに最適化
- **ポート競合ゼロ**: 各ワークスペースにランダムなポートを自動割り当て。`Address already in use: 5432` とはもうおさらば
- **リモート実行**: コンテナをリモート Docker ホストで動かし、ローカルマシンの負荷を軽減
- **自動クリーンアップ**: ワークスペース削除時に関連コンテナも即座に破棄

## インストール

### ソースからビルド

```bash
git clone https://github.com/your-org/aether.git
cd aether
cargo install --path .
```

### Cargo

```bash
cargo install aether
# バイナリ名は `ajj`（AI-jj の略）
```

## 使い方

### 1. プロジェクトで Aether を初期化

環境定義用の `aether.toml` を作成：

```toml
# aether.toml
[backend]
type = "docker"  # or "ssh"

[services.postgres]
image = "postgres:15"
ports = ["5432"]  # Aether がランダムなホストポートに自動マッピング
env = { POSTGRES_PASSWORD = "password" }

[injection]
file = ".env"
template = "DATABASE_URL=postgres://postgres:password@localhost:{{ services.postgres.ports.5432 }}/mydb"
```

### 2. ワークスペースを作成

```bash
# ワークスペース作成（コンテナも自動起動）
$ ajj workspace add feature-login-fix
> Creating workspace...
> Spawning postgres container (ID: 8a7f2b)... mapped to port 32891
> Injected config into .env
> Workspace ready at ./feature-login-fix

# 環境変数を読み込んでテスト実行
$ cd feature-login-fix
$ ajj run -- cargo test

# 作業完了
$ jj new main -m "fix: login bug"
$ jj squash
```

### 3. 後片付け

ワークスペース削除と同時にインフラも消える：

```bash
$ ajj workspace forget feature-login-fix
> Removing workspace...
> Killing container 8a7f2b...
> Cleaned up.
```

## コマンド一覧

| コマンド | 説明 |
|---------|------|
| `ajj workspace add <dest>` | ワークスペース作成＋コンテナ起動 |
| `ajj workspace forget <name>` | ワークスペース削除＋コンテナ停止 |
| `ajj run -- <command>` | 環境変数を読み込んでコマンド実行 |
| `ajj status [--json]` | ワークスペースとコンテナの状態を表示 |
| `ajj list [--json]` | 全ワークスペースを一覧表示 |
| `ajj cleanup [--force]` | 孤立コンテナを削除 |
| `ajj <jj-command>` | jj コマンドをそのまま実行 |

## アーキテクチャ

Aether は `jj` をラップし、ワークスペース操作をフックしてインフラ管理を自動化します。

```
┌─────────────────┐
│   AI Agent      │
└────────┬────────┘
         │ ajj workspace add feature-x
         ▼
┌─────────────────┐
│   Aether CLI    │──────────────────┐
└────────┬────────┘                  │
         │                           │
         ▼                           ▼
┌─────────────────┐         ┌─────────────────┐
│   Jujutsu (jj)  │         │   Docker API    │
│   VCS 操作      │         │   コンテナ管理  │
└─────────────────┘         └─────────────────┘
```

## 開発

### 必要なもの

- Rust 1.70+
- Docker
- [Jujutsu (jj)](https://github.com/martinvonz/jj)

### ビルド

```bash
cargo build --release
```

### テスト

```bash
cargo test
```

### Lint

```bash
cargo clippy -- -D warnings
cargo fmt --check
```

## コントリビュート

AI ソフトウェアエンジニアのための OS を一緒に作りませんか？

1. このリポジトリを Fork
2. フィーチャーブランチを作成 (`jj new main -m "feat: add amazing feature"`)
3. テスト実行 (`cargo test`)
4. Pull Request を送信

## ライセンス

MIT
