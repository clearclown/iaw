Aether (ajj) 🌌Infrastructure as Workspace.The missing link for parallel AI-driven development.Aether は Jujutsu (jj) のラッパーであり、バージョン管理ワークフローの第一級市民として インフラストラクチャのライフサイクル を扱います。AIエージェント (Claude Code, GitHub Copilot Workspace) の時代に向けて設計された Aether は、ポートの競合やリソースの枯渇を起こすことなく、数十の並列開発環境を即座に生成することを可能にします。🚀 コンセプト従来の開発では：ブランチ は安価で分離されています。環境 (DB, サーバー) は重く、共有され、競合が発生しやすいです。Aether (IaW) では：ajj workspace add を実行すると、ブランチ と 専用のコンテナ環境 が作成されます。インフラストラクチャは一時的です：ワークスペースと共に生き、ワークスペースと共に死にます。✨ 特徴🤖 AIネイティブデザイン: LLMエージェント向けに最適化された、構造化JSON出力と決定論的な環境プロビジョニング。⚡ ゼロ設定競合: すべてのワークスペースにランダムなポートを自動的に割り当てます。もう Address already in use: 5432 に悩まされることはありません。☁️ リモートオフロード: コンテナをリモートの Kubernetes クラスターや強力な Docker ホストで実行し、AI エージェントが並行して作業している間も、手元のラップトップを快適に保ちます。🧹 自動 GC: ワークスペースを削除すると、関連するコンテナも即座にキルされます。📦 インストールcargo install aether
# バイナリ名は `ajj` (AI-jj) です
📖 使用方法1. リポジトリで Aether を初期化する環境を定義するために aether.toml を作成します。# aether.toml
[backend]
type = "docker" # または "kubernetes", "ssh"

[services.postgres]
image = "postgres:15"
ports = ["5432"] # Aether はこれをランダムなホストポートにマッピングします
env = { POSTGRES_PASSWORD = "password" }

[injection]
file = ".env"
template = "DATABASE_URL=postgres://postgres:password@localhost:{{ services.postgres.ports.5432 }}/mydb"
2. AI エージェント向け (ワークフロー)AIエージェントには、git や素の jj の代わりに ajj を使うよう指示する必要があります。# 1. エージェントがタスク用の新しいワークスペースを作成する
$ ajj workspace add feature-login-fix
> 🌌 Creating workspace...
> 🚀 Spawning postgres container (ID: 8a7f2b)... mapped to port 32891
> 📝 Injected config into .env
> ✅ Workspace ready at ./feature-login-fix

# 2. エージェントがテストを実行する (自動的に隔離されたDBを使用)
$cd feature-login-fix$ ajj run -- cargo test

# 3. エージェントがタスクを完了する
$jj new main -m "fix: login bug"$ jj squash
3. クリーンアップワークスペースが削除されると、インフラストラクチャも消滅します。$ ajj workspace forget feature-login-fix
> 🗑️  Removing workspace...
> 💀 Killing container 8a7f2b...
> ✅ Cleaned up.
🏗️ アーキテクチャAether は jj バイナリをラップします。ワークスペースコマンドをインターセプトして、インフラストラクチャのフックをトリガーします。sequenceDiagram
    participant Agent as AI Agent
    participant Aether as ajj CLI
    participant JJ as Jujutsu VCS
    participant Docker as Remote Docker

    Agent->>Aether: workspace add feature-x
    Aether->>JJ: workspace add feature-x
    JJ-->>Aether: Success (path created)
    Aether->>Docker: Run containers (Namespace: feature-x)
    Docker-->>Aether: Ports: {5432 -> 32001}
    Aether->>Aether: Write .env (DB_PORT=32001)
    Aether-->>Agent: JSON Info { "status": "ready", ... }
🤝 コントリビューション私たちは AI ソフトウェアエンジニアのための OS を構築しています。ぜひ参加してください！📜 ライセンスMIT