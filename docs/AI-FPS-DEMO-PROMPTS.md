## 目标

用 **Rust + Bevy** 做一个“Doom/Wolf 走廊射击”风格的最小可玩 Demo，用来验证“纯代码引擎 + 多 Agent 工作流（制作人/策划/程序/美术/QA）”的可行性。

## 技术栈约束（必须满足）

- **引擎**：Bevy（纯代码、无编辑器依赖）
- **多 Agent**：按岗位拆分（制作人/策划/关卡/程序/美术/测试），并产出可审计交付物
- **Hermes**：消息/事件总线（跨岗位产物、门禁结果、日志）
- **Harness**：门禁与验收（读取规范文件，校验后才能运行/进入下一阶段）
- **资源**：只用 **CC0** 公开资源（贴图/字体/音效等），在 `assets/` 记录来源

## 公开素材来源（便于后续继续扩充）

- **OpenGameArt（CC0 音效/音乐/贴图）**：`https://opengameart.org/`
- **Google Noto CJK 字体（用于中文 UI）**：`https://github.com/googlefonts/noto-cjk`
- **Kenney（CC0 游戏素材/音效）**：`https://kenney.nl/assets`

## 最小玩法闭环（先做这个，不要发散）

- 第一人称移动：WASD + Shift
- 视角：键盘方向键（不依赖鼠标）
- 枪械：hitscan（射线检测）
- 敌人：占位模型 + 血量 + 追击 + 近战伤害 + 死亡
- HUD：HP + 当前层数
- 关卡：10 层；每层敌人递增；碰到出口进入下一层；第 10 层通关回主菜单

## 规范与门禁（OpenSpec 等价物）

用一个可机器读写的规范文件作为“关卡壳”：

- `assets/level_plan.yaml`
  - `player_start`
  - `pieces`（Floor/Wall/Door/Key/Light）

Harness gate（示例）：

- `pieces` 不能为空
- 必须至少包含一个 Floor
- 有 locked door 就必须有 Key
- 坐标/尺寸必须有限且在合理范围内

> 约定：任何 Agent 改动 `assets/level_plan.yaml` 都必须先通过 Harness gate，失败则返工。

## 多 Agent 组织方式（建议）

- **制作人（Producer Agent）**
  - 负责方向、门禁阈值、取舍
  - 输出：Gate 结论、下一步拆分任务
- **关卡/策划（Level Designer / Game Designer）**
  - 负责关卡壳 spec 与递进节奏
  - 输出：`assets/level_plan.yaml`、楼层递进规则
- **程序（Gameplay Programmer）**
  - 负责可玩闭环实现、可回归
  - 输出：Rust 代码 + 可运行 Demo
- **美术（Art Director / Technical Artist）**
  - 只选 CC0 资源，记录来源，保证风格一致
  - 输出：`assets/textures/*` + 来源文件
- **QA（QA Tester）**
  - 负责最小测试脚本与验收清单
  - 输出：测试步骤与关键指标（通关/卡死/帧率等）

## 给 AI 的执行提示词（可直接复用）

### 1) 制作人（方向与门禁）

- 目标：先验证“可玩闭环 + 门禁 + 10 层递进”，不追求特效与 UI 华丽
- 规则：任何新增功能必须能被演示并能回归验证，否则拒绝合并
- Gate：每次迭代必须能 `cargo run`，并能在 1 分钟内看到玩法循环（开火→命中→敌人追击→掉血→到出口进下一层）

### 2) 美术资源（CC0）

- 只允许 CC0；下载后写入 `assets/**/README_CC0.txt`（来源页面 + 文件名）
- 优先选择：seamless PNG（避免引擎 JPEG 支持问题）

### 3) 关卡递进

- 10 层；每层“障碍/敌人数量”递增，且布局必须 deterministic（seed + floor）
- 出口必须明显可见（发光/颜色区别）

### 4) 程序实现

- 玩家不能被随机障碍卡死：障碍生成要避开出生点与出口，并避免重叠
- 无输入时水平速度归零，防止碰撞残余速度让玩家“自己跑”

