# Aether (ajj)

**Infrastructure as Workspace** - The missing link for parallel AI-driven development.

Aether は [Jujutsu (jj)](https://github.com/martinvonz/jj) のラッパーであり、バージョン管理ワークフローの第一級市民として **インフラストラクチャのライフサイクル** を扱います。

AIエージェント (Claude Code, GitHub Copilot Workspace) の時代に向けて設計された Aether は、ポートの競合やリソースの枯渇を起こすことなく、数十の並列開発環境を即座に生成することを可能にします。

## コンセプト

**従来の開発では：**
- ブランチ は安価で分離されています
- 環境 (DB, サーバー) は重く、共有され、競合が発生しやすいです

**Aether (IaW) では：**
- `ajj workspace add` を実行すると、**ブランチ** と **専用のコンテナ環境** が作成されます
- インフラストラクチャは一時的です：ワークスペースと共に生き、ワークスペースと共に死にます

## 特徴

- **AIネイティブデザイン**: LLMエージェント向けに最適化された、構造化JSON出力と決定論的な環境プロビジョニング
- **ゼロ設定競合**: すべてのワークスペースにランダムなポートを自動的に割り当て。`Address already in use: 5432` に悩まされることはありません
- **リモートオフロード**: コンテナをリモートの Docker ホストで実行し、手元のラップトップを快適に保ちます
- **自動 GC**: ワークスペースを削除すると、関連するコンテナも即座にキルされます

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
# バイナリ名は `ajj` (AI-jj) です
```

## 使用方法

### 1. リポジトリで Aether を初期化する

環境を定義するために `aether.toml` を作成します：

```toml
# aether.toml
[backend]
type = "docker"  # または "ssh"

[services.postgres]
image = "postgres:15"
ports = ["5432"]  # Aether はこれをランダムなホストポートにマッピングします
env = { POSTGRES_PASSWORD = "password" }

[injection]
file = ".env"
template = "DATABASE_URL=postgres://postgres:password@localhost:{{ services.postgres.ports.5432 }}/mydb"
```

### 2. ワークスペースを作成する

```bash
# 新しいワークスペースを作成（コンテナも自動起動）
$ ajj workspace add feature-login-fix
> Creating workspace...
> Spawning postgres container (ID: 8a7f2b)... mapped to port 32891
> Injected config into .env
> Workspace ready at ./feature-login-fix

# 環境変数をロードしてテスト実行
$ cd feature-login-fix
$ ajj run -- cargo test

# 作業完了後
$ jj new main -m "fix: login bug"
$ jj squash
```

### 3. クリーンアップ

ワークスペースが削除されると、インフラストラクチャも消滅します：

```bash
$ ajj workspace forget feature-login-fix
> Removing workspace...
> Killing container 8a7f2b...
> Cleaned up.
```

## コマンド一覧

| コマンド | 説明 |
|---------|------|
| `ajj workspace add <dest>` | ワークスペース作成 + コンテナ起動 |
| `ajj workspace forget <name>` | ワークスペース削除 + コンテナ停止 |
| `ajj run -- <command>` | 環境変数をロードしてコマンド実行 |
| `ajj status [--json]` | ワークスペースとコンテナの状態表示 |
| `ajj list [--json]` | 全ワークスペース一覧 |
| `ajj cleanup [--force]` | 孤立コンテナの削除 |
| `ajj <jj-command>` | jj コマンドへのパススルー |

## アーキテクチャ

Aether は `jj` バイナリをラップし、ワークスペースコマンドをインターセプトしてインフラストラクチャのフックをトリガーします。

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
│   VCS操作       │         │   コンテナ管理   │
└─────────────────┘         └─────────────────┘
```

## 開発

### 必要条件

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

## コントリビューション

私たちは AI ソフトウェアエンジニアのための OS を構築しています。ぜひ参加してください！

1. Fork this repository
2. Create your feature branch (`jj new main -m "feat: add amazing feature"`)
3. Run tests (`cargo test`)
4. Submit a Pull Request

## ライセンス

MIT
