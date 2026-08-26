# sshboard

> このリポジトリは、人と AI エージェント（Claude Code）が一緒に開発することを前提に構成されています。
> AI エージェントは以下を必ず守ってください。

## 必読

- **`.claude/rules/product-baseline.md`** — 開発のベースルール。**最優先で従うこと**。
- **`PRD.md`** — このプロダクトの方向性・仕様。
- **`.claude/roadmap.md`** — フェーズと進め方。**いまは Phase 0**。
- **`.claude/decisions.md`** — 決定と、その理由（D1〜D14）。**未決は D6 / D10 のみ**。
- **`.claude/issues/`** — 着手すべきローカル Issue。

## この製品に固有の禁止事項（最優先）

1. **`run_command(cmd)` 相当の MCP ツールを作らない。**
   引数で任意の文字列をシェルへ渡す口を 1 つも作らない（D3）。
   必要な操作は用途別ツールか、許可リスト方式の `run_readonly` に足す。
2. **Phase 1 に書き込みを入れない。**アップロード・上書き・削除・リネーム・移動・
   パーミッション変更・サービス再起動・パッケージ操作・sudo（D2）。
3. **裏で見えない SSH セッションを張らない。**GUI と MCP は同じ Operation Engine と
   同じ 1 本の SSH を共有する（PRD §4-1）。**見えないことが最大の危険**。
4. **接続先を成果物へ書かない。**ホスト名 / IP / ユーザー名 / パス。
   コード・文書・commit message・**スクリーンショットとデモ GIF** すべて。
   **この製品は画面そのものに接続先が写り込む。**
5. **自前の鍵ストアを作らない。**OS 資格情報ストアと ssh-agent へ委譲する（D11）。
   `list_connections` は接続の識別子だけを返し、**認証情報を返さない**。

## 守ることの要点（詳細は product-baseline.md）

- フロント側は **pnpm のみ使用**（npm / yarn 禁止）。Rust 側は cargo。
- 実装前に計画を立てる。小さいフェーズで作業。**テストを後回しにしない／落ちるテストを消さない。**
- **commit は AI、push は人間。**人間の確認なしに push しない。
- 秘密情報（鍵・パスフレーズ・トークン）は **AI が作らない・置かない・貼らない。**
- 進捗は `.claude/project-status.md` に随時記録。**テストが無い状態で「完了」と書かない。**
- **public リポジトリです。**コード・文書・commit history に個人名・個人メールアドレスを
  残さないこと（`.github/workflows/oss-privacy-check.yml` が検出します）。

## いまやること — Phase 0

**`.claude/issues/001` が最優先。**ここが成立しないなら製品が成立しません。

| # | 未知 |
|---|---|
| 001 | Tauri 2 に MCP を同居させ、MCP 呼び出しが GUI の帯へリアルタイムに出るか |
| 002 | 稼働中サーバーの SSH に繋がるか（古い鍵交換方式・暗号方式） |
| 003 | SSH ライブラリの選定（D6・**カタログで決めない**） |
| 004 | 資格情報を OS ストア / ssh-agent から読む（Windows / macOS 両方） |
| 005 | `tail -f` を GUI と MCP へ同時に流す／ANSI とプレーンの分離 |

**Phase 0 が通らなければ archive します。**通らないものが残った状態で Phase 1 の
機能を書き始めないこと。

## 進捗管理

- `.claude/project-status.md` … 現在フェーズ・完了/未完了・次タスク・既知問題
- `.claude/decisions.md` … 技術的決定と、その理由

## 日課（AI エージェント向け）

### セッション開始時

1. `git pull --ff-only`
2. `gh issue list --state open` ＋ `gh issue list --state closed --limit 10`
   （**close 済 Issue にも後追いで指示や訂正が入ることがあるため、必ず両方見る**）
3. open Issue は全件 `gh issue view <番号> --json title,body,comments,author,createdAt --jq '.'` で本文＋コメントを確認
   （`--comments` は出力が空のまま exit 0 することがあり、「読んだが何も無かった」と区別できないため使わない）
4. 何を確認し、どれから着手するかを返す

### Issue への反応（着手前）

- 新規 Issue・新規コメントには、**着手前に最低 1 回反応する**（「読みました。〇〇から着手します。」）
- **沈黙は「読んでいない」「止まっている」「無視した」と区別がつきません。**
- 「承知しました」だけを返さない。**できていないなら、できていないと認め、対策と日付を出す。**
  前提がおかしいと思うなら異議・代案を出す。

### セッション終了時

1. `.claude/project-status.md` に進捗を記録（テストが無い状態で「完了」と書かない）
2. 完了した Issue は `gh issue close <番号> --comment "..."`

### git pull の 3 タイミング

1. **セッション開始時**: `git pull --ff-only`
2. **commit する直前**: `git pull --rebase --ff-only`
3. **人間が push するとき**: ff エラーなら `pull --rebase` してから再 push
