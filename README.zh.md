## Bevy + AI Wolf3D Demo（验证项目）

**语言 / Languages**: [中文](README.zh.md) | [日本語](README.ja.md) | [English](README.en.md)

## 目的与结论

本项目用于验证：**Bevy + AI（自然语言提示词驱动）是否能做出一款可玩的 3D FPS Demo**。  
结论：**可行，且已完成验证**（实现了可玩闭环、UI 点击/场景切换、贴图/适配思路、音频、射线命中、贴花、受击、帧动画控制、武器/道具/血量等常规功能）。

项目地址（发布/继续完善）：`git@github.com:sconi789/BevyAiWolfenstein.git`

## 游戏操作方式（以代码为准）

- **移动**：W / A / S / D
- **冲刺**：Left Shift
- **转向/俯仰**：方向键 ← → ↓ ↑（键盘视角）
- **射击**：Space（按住连射）
- **开门/交互**：E
- **切换武器**：1（手枪）、2（霰弹枪，解锁后）
- **使用血包**：H（库存 > 0 且 HP 未满）
- **小地图**：M（当前代码里有切换键；若 UI 未接入则忽略）
- **调试**：
  - F1：敌人预览开关
  - `[` / `]`：预览帧切换

## 项目架构（Rust / Bevy / Hermes / Harness）

- **Rust + Bevy 0.14**：核心引擎与渲染/UI/音频
- **bevy_rapier3d**：3D 物理与射线检测（hitscan）
- **`crates/hermes`（Hermes）**：轻量事件总线，统一记录“哪个角色（Producer/Designer/Programmer/Art/QA）在什么主题（Gate/Gameplay/QA…）下做了什么决定/产出”，便于复盘与避免走回头路
- **`crates/harness`（Harness）**：门禁/验收。启动时读取并校验 `assets/level_plan.yaml`，不通过则直接失败，保证“规范优先、可运行优先”
- **资源规范**：所有 CC0 素材放入 `assets/`，并在对应目录的 `README_CC0.txt` 记录来源与文件名

相关文件：
- **门禁规范**：`assets/level_plan.yaml`（由 Harness 校验）
- **提示词/工作流沉淀**：`docs/AI-FPS-DEMO-PROMPTS.md`
- **游戏内说明（同内容三语）**：`docs/ABOUT.zh.md` / `docs/ABOUT.ja.md` / `docs/ABOUT.en.md`

## Hermes 如何把控方向、避免重复出错（实践要点）

- **把“方向/门槛/门禁结论”变成事件流**：用 `HermesEvent { topic, from, message }` 把关键决策显式写入日志（例如 ProducerGate：某项改动是否允许合并、当前 Demo 的最小闭环是什么）
- **把“可回归”写进提示词**：每次改动必须能 `cargo run`，并能在 1 分钟内验证关键闭环；不满足则回滚/拒绝
- **减少隐性沟通成本**：当 AI 在不同“角色”间切换时，Hermes 的事件日志相当于统一的“交付物与结论”载体

## AI 如何找到免费资源并应用（CC0）

实践约束：
- 只允许 **CC0**（音效/音乐/贴图/字体）
- 下载后必须补充 `assets/**/README_CC0.txt`（来源页面 + 文件名）

常用来源（示例）：
- OpenGameArt（CC0）：`https://opengameart.org/`
- Kenney（CC0）：`https://kenney.nl/assets`
- Noto CJK 字体：`https://github.com/googlefonts/noto-cjk`

## 用提示词让 AI 调整动画序列帧匹配（要点）

在该 Demo 里，序列帧/UV 匹配的关键是：**把“帧编号 → UV 区间”与“镜像/翻转”等适配规则明确成可操作的约束**，然后让 AI 按约束迭代，直到画面与预期一致。  
建议把提示词写成：

- 资源信息：贴图尺寸、行列数、每帧宽高、首帧位置
- 规则：帧从 0 还是 1 开始、是否需要 v-flip、每帧持续时间
- 验证方式：给出“按键切帧/预览开关”，肉眼确认序列是否正确

（项目里已有按键切帧的预览能力：F1 开关，`[` / `]` 切帧。）

