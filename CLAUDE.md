# CLAUDE.md - Aether Project Guide

## Project Overview

**Aether (ajj)** は "Infrastructure as Workspace" (IaW) を実現する Jujutsu (jj) VCS のラッパーCLIツールです。AIエージェントによる並列開発を可能にするため、ワークスペース（コードブランチ）とインフラ（コンテナ環境）のライフサイクルを同期させます。

## Core Concepts

- **IaW (Infrastructure as Workspace)**: ワークスペースとインフラのライフサイクルを1対1で結合
- **動的ポートマッピング**: ポート競合を回避するため、空きポートを動的に割り当て
- **コンテキスト注入**: 環境変数やDB接続情報を自動で `.env` に注入
- **AI-First設計**: 構造化JSON出力、決定論的な環境プロビジョニング

## Tech Stack

- **言語**: Rust
- **バイナリ名**: `ajj`
- **VCS**: Jujutsu (jj) をラップ
- **バックエンド**: Docker (Local/Remote), Kubernetes (将来)

## Project Structure

```
IaW/
├── docs/
│   ├── 企画書.md       # プロジェクトのビジョンと戦略
│   └── 要件定義書.md   # 機能・非機能要件の詳細
├── README.md           # プロジェクト概要とクイックスタート
├── aether.toml         # ワークスペース設定ファイル（ユーザー作成）
└── CLAUDE.md           # 本ファイル
```

## Key CLI Commands

```bash
# ワークスペース作成（インフラも同時起動）
ajj workspace add <destination>

# 環境変数をロードしてコマンド実行
ajj run -- <command>

# 状態確認（JSON出力対応）
ajj status --json

# ワークスペース削除（インフラも同時破棄）
ajj workspace forget <workspace>
```

## Configuration Format (aether.toml)

```toml
[backend]
type = "docker"  # "docker", "kubernetes", "ssh"

[services.postgres]
image = "postgres:15"
ports = ["5432"]  # 動的にホストポートへマッピング
env = { POSTGRES_PASSWORD = "password" }

[injection]
file = ".env"
template = "DATABASE_URL=postgres://postgres:password@localhost:{{ services.postgres.ports.5432 }}/mydb"
```

## Architecture

Aether は「Sidecar / Wrapper」アーキテクチャを採用:

1. **CLI (ajj)**: ユーザー/AIエージェントからのコマンドを受け付け
2. **Jujutsu委譲**: VCS操作は内部で `jj` に委譲
3. **Resource Provisioner**: バックエンド（Docker/K8s）へのコンテナ管理
4. **Context Injection**: 接続情報を `.env` などに注入

## Development Guidelines

### AI-First UX

- エラーメッセージは構造化された情報を含め、AIが原因特定・修正できるようにする
- 対話モードより引数による完全制御を優先
- JSON/XML 形式の標準出力をサポート

### Performance Requirements

- `workspace add` のオーバーヘッドはコンテナ起動時間を除き1秒以内
- `jj` のネイティブな高速性を損なわない

### Code Style

- Rust の標準的なスタイル (`rustfmt`, `clippy`)
- エラーハンドリングは `Result` 型を活用
- バックエンドはプラグイン可能な設計（trait による抽象化）

## Roadmap

1. **Phase 1 (MVP)**: Local Docker ラッパー、動的ポートマッピング
2. **Phase 2**: Remote Docker/SSH、Kubernetes Namespace 対応
3. **Phase 3**: MCP サーバー実装、AIエージェント向け標準プロトコル

## Testing

```bash
# テスト実行
cargo test

# ワークスペース内でのテスト（隔離環境使用）
ajj run -- cargo test
```
