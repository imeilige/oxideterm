# 删除 AI 助手的执行计划

> 目标：移除 OxideSens 助手，释放 `__text` 体积，并把 AI 状态从 workspace 各子系统里彻底摘干净。
> 依据：[AI.md](AI.md) 的耦合地图。规模：`sidebar/ai` 36,141 + `ai_state` 10,806 + `ai_runtime_context` 1,109 + 6 个 crate 约 60,591 行。

## 0. 为什么必须分批，以及分批的原则

`ai_entity` 在 **30 多个文件、约 178 处**被外部引用。一次性删除会同时打断 4 类互不相干的行为：

| 类别 | 代表位置 | 删掉后的行为变化 |
|---|---|---|
| IME 命中测试 | `ime.rs`（32 处） | 聊天框不再有光标定位分支 |
| 设置页输入归属 | `settings/cards.rs`（19 处） | 不再有 AI 独占的设置输入框 |
| 模态与脏状态守卫 | `root/modal_owner.rs`、`actions.rs`、`history_quit.rs` | 退出/切换时不再被 AI 草稿拦截 |
| 上下文注入 | `pane_tree.rs`、`connection_monitor/`、`remote_desktop/` | 主机信息、截图不再喂给模型 |

**原则：每批只动一个目录、每批单独编译、每批单独 commit、每批单独验证。**任何一批出问题就单独回滚那一批，不影响已完成的批次。

## 1. 批次 0：完成侧栏状态解耦（剩余 15 处）

### 为什么先做这个

`context_sidebar_width` / `context_sidebar_resizing` 现在有两份：`WorkspaceApp` 已有（`83eeb2b9f` 迁入），但 15 处读取仍走 `ai_entity.read(cx).chat_ui().sidebar_*`。**同一个状态两个来源**，这是必须先收口的技术债。

### 改动清单

| 文件 | 行 | 替换 |
|---|---|---|
| `session_manager/views.rs` | 587 | `self.ai_entity…sidebar_width` → `self.context_sidebar_width` |
| `connection_monitor/health/packages.rs` | 873 | 同上 |
| `connection_monitor/health/scheduled_tasks.rs` | 1969 | 同上 |
| `connection_monitor/health/tmux.rs` | 18 | 同上 |
| `connection_monitor/health/ports.rs` | 768 | 同上 |
| `connection_monitor/health/monitor.rs` | 39 | 同上 |
| `connection_monitor/health/filesystems.rs` | 915 | 同上 |
| `connection_monitor/health/process.rs` | 1150 | 同上 |
| `connection_monitor/health/logs.rs` | 12 | 同上 |
| `tabs/detach.rs` | 145 | 同上 |
| `tabs/render.rs` | 994 | 同上 |
| `tabs/navigation.rs` | 1280 | 同上 |
| `browser_behavior.rs` | 462, 539 | 字段 `ai_sidebar_resizing` → `context_sidebar_resizing`，值改读新字段 |
| `root/render.rs` | 235 | `self.context_sidebar_resizing` |
| `sidebar/ai/input.rs` | 359 | 随批次 2 一起删，本批先改 |

### 验收

```bash
grep -rc "chat_ui().sidebar_width\|chat_ui().sidebar_resizing" \
  crates/oxideterm-gpui-app/src --include=*.rs | grep -v ":0"
# 期望：批次 0 完成后只剩 sidebar/ai/input.rs 一处，批次 2 删除后归零

cargo check -p oxideterm-gpui-app --all-targets   # 0 error
cargo test -p oxideterm-gpui-app
```

### 收益

批次 0 本身不删代码，但它是后续所有批次的前提：把 15 处跨目录读取变成读 workspace 自己的字段后，`connection_monitor`（8 处）、`tabs`（3 处）、`session_manager` 就**不再依赖 AI**，批次 3/4 可以整目录删除。

## 2. 批次 1：`connection_monitor`（8 文件 / 8 处）

### 为什么放这里

`connection_monitor/health/` 下 9 个文件各只有 1 处引用，模式完全一致——都是把 `sidebar_width` 喂给采样结果的布局计算。批次 0 完成后这 8 处会全部变成 `self.context_sidebar_width`，**本批次就变成纯粹的机械替换**。

### 改动

批次 0 完成后，逐文件把 `self.ai_entity.read(cx).chat_ui().sidebar_width` 替换为 `self.context_sidebar_width`。

### 验收

```bash
grep -rn "ai_entity" crates/oxideterm-gpui-app/src/workspace/connection_monitor/  # 期望 0
cargo check -p oxideterm-gpui-app --all-targets
```

## 3. 批次 2：`sidebar/ai` + `ai_state` + `ai_runtime_context`（主体，约 48,000 行）

### 这是最大的一批，也是最独立的一批

关键事实：**560 处引用都在 `sidebar/ai/` 目录内部**，整体删除即可。外部只剩：
- `settings/ai/*`（AI 设置页，随本批删）
- `sidebar/ai/input.rs` 的最后 1 处宽度读取（随本批删）

### 步骤

1. 删 `src/workspace/sidebar/ai.rs` + `src/workspace/sidebar/ai/`
2. 删 `src/workspace/ai_state.rs` + `src/workspace/ai_state/`
3. 删 `src/workspace/ai_runtime_context/`
4. 删 `src/workspace/settings/ai/` + `settings/ai_page.rs`
5. 从 `workspace.rs` 移除 `mod` 声明与 5 个字段：
   `ai_entity`、`_ai_entity_subscription`、`ai_text_editor`、`ai_text_editor_dialog`、`ai_rag`
6. 从 `sidebar.rs` 移除 `mod ai;` 及其 `pub use ai::{…}` 块
7. 从 `settings.rs` 移除 `mod ai_page;` 及其 re-export

### 预期残留错误（下一批处理）

`ai_entity` / `AiWorkspaceEntity` 仍被 `ime.rs`(32)、`settings/cards.rs`(19)、`root/*`(37)、`actions.rs`(16)、`pane_tree.rs`(8)、`tabs/*`(17) 等引用 → 约 150 个错误。

### 验收

```bash
cargo check -p oxideterm-gpui-app 2>&1 | grep -cE ": error"
# 期望：0 error 在本批不可能（外部引用还在），但错误数应从 ~900 降到 ~150
# 且错误应全部是 "no field `ai_entity`" / "cannot find `ai_state`"
```

## 4. 批次 3：`settings`（10 文件 / 70 处）

### 分类

| 文件 | 处数 | 内容 |
|---|---|---|
| `settings/cards.rs` | 19 | 输入归属判定、模型选择器、IME target 分派 |
| `settings/ai/*` | 36 | 随批次 2 已删 |
| `settings/terminal_controls.rs` | 8 | 终端设置里的模型选择 |
| `settings/surface.rs` / `controls.rs` / `search.rs` | 4 | 设置页索引与搜索条目 |

### 关键改动

`cards.rs:691` 的 `focused_settings_input()` 分派是核心：它决定一个 `SettingsInput` 归设置实体还是 AI 实体。AI 删除后，所有输入归设置实体，`owns_settings_input()` 整条分支消失。

同时要清理 `SettingsTab` 里的 AI 页（`AiSettingsPage` 枚举）及相关 `SettingsInput` 变体——这两个在 `oxideterm-settings-model` crate 里，属于本批的连带范围。

### 验收

```bash
grep -rn "ai_entity\|AiSettingsPage\|AiSettingsViewSection" \
  crates/oxideterm-gpui-app/src/workspace/settings/ crates/oxideterm-settings-model/src/
cargo check -p oxideterm-gpui-app --all-targets
```

## 5. 批次 4：`ime.rs` + `selectable_text.rs`（34 处）

### 改动

**删 4 个 IME target 变体**（`ime.rs`）：

```rust
AiInlinePrompt, AiChatInput, AiConversationRename, AiMessageEdit,
```

连带删除它们的：
- `anchor_id()` 映射（`:515-518` 的 1896-1899）
- 焦点解析分支（`active_ime_target()`）
- 换行策略分支（`ime_target_accepts_newline`）
- 行高分支（`ime_target_line_height` 的 `px(20.0)`）
- 水平内边距分支（`ime_target_horizontal_padding` 的 `px(0.0)`）
- `current_value_for_target()` 的取值臂
- `replace_ime_target_text()` 的写入臂
- 垂直导航（`ai_edit_vertical_destination`）
- 自动补全（`ai_chat_autocomplete_items`）

`selectable_text.rs` 的 2 处是 markdown 链接类型引用，随 markdown 一起处理。

### 注意

这批的分支都是**控制流**（match 臂、`if/else` 链），不能用正则批量删。必须逐个读上下文、手工确认删掉的是 AI 分支而不是相邻的通用分支。

### 验收

```bash
grep -n "AiChatInput\|AiMessageEdit\|AiInlinePrompt\|AiConversationRename" \
  crates/oxideterm-gpui-app/src/workspace/ime.rs   # 期望 0
cargo check -p oxideterm-gpui-app --all-targets
```

## 6. 批次 5：`root` + `actions` + 杂项（约 60 处）

| 文件 | 处数 | 内容 |
|---|---|---|
| `root/render.rs` | 9 | 悬浮聊天层、模型选择器浮层、内联提示面板 |
| `root/modal_owner.rs` | 5 | 聊天确认框的模态所有权 |
| `root/init.rs` | 7 | AI 实体构造与订阅 |
| `actions.rs` | 16 | 确认对话框动作、退出拦截、命令面板 |
| `pane_tree.rs` | 8 | 上下文选择与截图 |
| `tabs/*` | 17 | tab 注册、焦点撤销 |
| `acp_workspace.rs` | 5 | ACP 会话 |
| `history_quit.rs` | 4 | 退出时的草稿守卫 |
| `remote_desktop/session.rs` | 4 | 远程桌面截图喂模型 |
| `cloud_sync/*` | 3 | 云同步事件 |
| `session_manager/*` | 5 | 会话树 |
| `terminal_git.rs` / `terminal_context_actions.rs` / `disclosure_motion.rs` | 4 | 杂项 |

### 验收

```bash
grep -rn "ai_entity\|ai_state\|ai_runtime_context\|AiWorkspace" \
  crates/oxideterm-gpui-app/src --include=*.rs | grep -v ":0"
# 期望：0
cargo check --workspace --all-targets
```

## 7. 批次 6：删除 6 个 crate

```
oxideterm-ai            48,961
oxideterm-public-mcp     6,576
oxideterm-acp-adapter    2,433
oxideterm-acp-host-tools    630
oxideterm-ai-tasks       1,173
oxideterm-skills           818
```

### 前置条件

- 批次 2-5 全部完成，app 侧不再引用
- `oxideterm-cli` 移除 AI 相关命令（`oxideterm-ai` 是 CLI 的依赖，必须一起处理）
- `oxideterm-settings` / `oxideterm-settings-model` 移除 AI 配置段
- `oxideterm-gpui-settings-view` 移除 `ai.rs`

### 验收

```bash
grep -rn "oxideterm_ai\|oxideterm-acp\|oxideterm-public-mcp\|oxideterm-skills" \
  crates/*/Cargo.toml   # 期望 0
cargo check --workspace --all-targets
```

## 8. 每批通用检查清单

```bash
# 1. 编译
cargo check --workspace --all-targets

# 2. 测试（全 workspace，不是单 crate——单 crate 会暴露时序敏感的假失败）
cargo test --workspace

# 3. i18n（删除了设置项/文案后必须同步）
python3 scripts/quality/audit_i18n.py
#   "Keys missing from locale catalogs" 必须为 0
#   "Source-used keys absent from every locale" 必须为 0
cargo test -p oxideterm-i18n

# 4. 删除的 i18n 键要 11 语言同步删
#    审计脚本只检查"源码引用是否存在"，不检查"未引用的键是否残留"，
#    所以残留键需要单独按 §AI.md 的清单清理

# 5. 格式（只格式化本批改动的文件，不要 cargo fmt --all）
rustfmt --edition 2024 <本批文件列表>

# 6. 提交
git add -u && git commit
```

## 9. 风险与回滚

| 风险 | 表现 | 应对 |
|---|---|---|
| 正则误删相邻分支 | 编译错误但位置诡异 | 逐文件 `git diff` 核对，不批量 |
| 删掉相邻的通用分支 | 能编译但行为变了 | 每批看 diff 而不是只看编译结果 |
| 单 crate 测试的时序假失败 | `mixed_pages_keep_identity…` 单独跑失败、全 workspace 通过 | 以 `cargo test --workspace` 为准 |
| i18n 残留键 | 界面显示裸键名 | 每批跑审计 + 手工清未引用键 |

**回滚单位是单个 commit。** 每批一个 commit，任何一批出问题：

```bash
git revert <该批 commit>
```

不会影响已完成的批次。

## 10. 预期收益

| 阶段 | 累计删除 | DMG 预估减少 |
|---|---|---|
| 批次 0-1 | 0 行（纯解耦） | 0 |
| 批次 2 | ~48,000 行 | ~4-5 MB |
| 批次 3-5 | ~5,000 行 | ~1 MB |
| 批次 6 | ~60,600 行（crate） | ~5-7 MB |
| **合计** | **~114,000 行** | **~10-13 MB** |

DMG 预估基于实测压缩特征：删除的是 `__text` 编译代码，压缩率约 2.3 倍（对比 `__const` 数据的 7.8 倍）。

**注意**：`__text` 会从 51.3 MB 降到 30 MB 左右，但 DMG 的边际收益取决于剩余内容的可压缩性，最终数字需要重新打包实测。

## 11. 前置依赖：先验证收益

批次 0 到 6 是 6 次提交、约 6-8 小时的机械工作，预期 DMG 再降 10-13 MB。

**建议先做一次打包实测**（当前基线 DMG 45.2 MB），确认：

- 45.2 MB 是否已可接受
- 10-13 MB 的预期收益是否值得这个工作量
- 有没有更省力的体积优化点（如 [INTEGRATION.md §4.2](INTEGRATION.md#42-二进制段构成实测) 列出的依赖大户）
