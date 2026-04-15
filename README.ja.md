## Bevy + AI Wolf3D Demo（検証プロジェクト）

**言語 / Languages**: [中文](README.zh.md) | [日本語](README.ja.md) | [English](README.en.md)

## 目的と結論

本プロジェクトは、**Bevy + AI（自然言語プロンプト駆動）で、遊べる 3D FPS デモを作れるか**を検証するためのものです。  
結論：**可能であり、検証は達成**（プレイアブルループ、UI クリック/ステート切替、テクスチャ適応の考え方、音声、レイ命中、デカール、被弾、フレームアニメ、武器/アイテム/HP などを実装）。

リポジトリ：`git@github.com:sconi789/BevyAiWolfenstein.git`

## 操作方法（コード準拠）

- **移動**：W / A / S / D
- **ダッシュ**：Left Shift
- **視点（Yaw/Pitch）**：矢印キー ← → ↓ ↑
- **射撃**：Space（押しっぱなしで連射）
- **ドア/インタラクト**：E
- **武器切替**：1（ピストル）、2（ショットガン：解禁後）
- **メディキット使用**：H（在庫 > 0 かつ HP 未満）
- **ミニマップ**：M（現在コード側にトグルあり。UI 側未接続なら無視）
- **デバッグ**：
  - F1：敵プレビュー切替
  - `[` / `]`：プレビューのフレーム切替

## アーキテクチャ（Rust / Bevy / Hermes / Harness）

- **Rust + Bevy 0.14**：ゲーム本体（描画/UI/音声）
- **bevy_rapier3d**：物理＆レイキャスト（hitscan）
- **`crates/hermes`（Hermes）**：軽量イベントバス。役割（Producer/Designer/Programmer/Art/QA）× トピック（Gate/Gameplay/QA…）で、意思決定と成果をログに残す
- **`crates/harness`（Harness）**：ゲート/検証。起動時に `assets/level_plan.yaml` を読み取り検証し、失敗なら即停止（仕様優先・実行可能性優先）
- **アセット規約**：CC0 素材は `assets/` に置き、`assets/**/README_CC0.txt` に出典を記録

関連：
- `assets/level_plan.yaml`（Harness が検証）
- `docs/AI-FPS-DEMO-PROMPTS.md`（プロンプト/進め方）
- `docs/ABOUT.*.md`（ゲーム内の三言語説明）

## Hermes で方向性を守り、手戻りを減らす

- **方向/基準をイベントにする**：ProducerGate などで「何を許可/拒否したか」を明文化
- **“回帰できる”をプロンプトに埋め込む**：毎回 `cargo run` と短時間チェックを必須にする
- **ロール間の受け渡しを簡潔に**：役割が変わっても、Hermes ログが“共通の結論と根拠”になる

## AI に CC0 素材を探させ、適用する（要点）

- CC0 のみ採用
- 取得したら `assets/**/README_CC0.txt` に「URL + ファイル名」を必ず追記

代表的な入手先：
- OpenGameArt：`https://opengameart.org/`
- Kenney：`https://kenney.nl/assets`
- Noto CJK：`https://github.com/googlefonts/noto-cjk`

## プロンプトでフレームアニメ（UV）を合わせる（要点）

重要なのは、**「フレーム番号 → UV 範囲」や「v-flip/並び順/持続時間」を制約として明確化し、プロンプトで反復する**ことです。  
検証しやすいように、F1 のプレビュー切替や `[` / `]` のフレーム切替のような“目視確認用の手段”を用意し、合うまで調整します。

