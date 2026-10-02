# AI 子系统（OxideSens）

> 对应 v2.2.8（`83eeb2b9f`）。

## 1. 四层结构与依赖方向

```
oxideterm-ai              纯逻辑，零 GPUI 依赖（Cargo.toml 无 gpui）
    ↑                     providers/ streaming/ rag/ acp/ mcp/ policy/ runtime_context/
    │
ai_runtime_context/       GPUI 侧：句柄签发与撤销的中枢（1,109 行）
    ↑
ai_state.rs + ai_state/  GPUI 侧：对话状态实体（6,882 + 3,924 行）
    ↑
sidebar/ai.rs + sidebar/ai/   GPUI 侧：面板 UI（190 + 36,141 行）
    ↑
WorkspaceApp 各个功能模块
```

依赖严格单向向下。`oxideterm-ai` 可以脱离 GPUI 单测，这也是它 48,961 行能有完整测试覆盖的原因。

### 1.1 `oxideterm-ai` 内部模块

| 模块 | 职责 |
|---|---|
| `providers/` | 多供应商接入（OpenAI 兼容、自定义端点） |
| `streaming/` `stream_state/` | SSE 流式响应解析与增量状态 |
| `orchestrator.rs` | 工具调用循环（模型 → 工具 → 结果 → 模型） |
| `policy.rs` | 工具审批策略、作用域判定 |
| `rag/` `provider_embeddings.rs` | 知识库检索与向量嵌入 |
| `acp/` | Agent Client Protocol 会话 |
| `mcp/` | Model Context Protocol 客户端 |
| `runtime_context/` | 运行时能力与句柄签发（见 §3） |
| `context_sanitizer.rs` | 送入模型前的上下文清洗 |
| `context_window.rs` | 上下文窗口预算 |
| `persistence/` | 对话与记忆落盘 |
| `key_store.rs` | 供应商密钥（走系统钥匙串） |

## 2. 为什么 `runtime_context` 是独立的

`crates/oxideterm-ai/src/runtime_context/identity.rs:227` 定义了六种可被 AI 操作的所有者：

```rust
pub enum RuntimeOwnerKind {
    LocalShell, Terminal, SshNode, SftpSession, IdeSurface, AppSurface,
}
```

`ai_runtime_context` 实体（`crates/oxideterm-gpui-app/src/workspace/ai_runtime_context/entity.rs`）为每种所有者维护一个注册项：

```rust
struct TerminalRuntimeOwner   { key: RuntimeOwnerKey, generation: RuntimeOwnerGeneration }
struct LocalShellRuntimeOwner { … }
struct NodeRuntimeOwner       { key, generation, connection_id }
struct SftpRuntimeOwner       { key, generation, connection_id, session_generation }
```

`generation` 是防 ABA 的版本号：面板拆了又建，旧的撤销动作不能误杀新注册的句柄。

**分离的理由**：句签发是安全边界（模型能操作什么），对话状态是功能状态。混在一起意味着改对话 UI 就会碰到授权逻辑。

## 3. 句柄签发流程

模型不能直接操作终端。它拿到的是一个**不透明句柄**：

1. `register_*` 在资源创建时注册所有者（`tabs/state.rs:register_tab_surface`、`tabs/nodes.rs:register_node_connection`）
2. 模型返回工具调用时引用句柄
3. `issue_app_surface_handle(tool_session, tab_id)` 校验所有者仍存在且 generation 匹配，签发句柄
4. `validate_terminal_handle` 在路由前再次校验
5. 资源销毁时 `revoke_*`，同代或更旧的句柄立即失效

`crates/oxideterm-ai/src/runtime_context/capability.rs` 定义能力（capability），`registry.rs` 管理注册表，`projection.rs` 负责把内部状态投影成模型可见的视图。

## 4. 耦合地图（重要）

`AiWorkspaceEntity`（字段名 `ai_entity`）在工作区里的引用分布：

| 子系统 | 文件数 | 引用处数 |
|---|---|---|
| `sidebar/ai/` 自身 | 30 | 560 |
| `settings/` | 10 | 70 |
| `ime.rs` | 1 | 32 |
| `root/` | 4 | 22 |
| `actions.rs` | 1 | 11 |
| `pane_tree.rs` | 1 | 5 |
| `acp_workspace.rs` | 1 | 5 |
| `remote_desktop/session.rs` | 1 | 4 |
| `history_quit.rs` | 1 | 4 |
| `connection_monitor/health/` | 8 | 8 |
| `tabs/` | 3 | 3 |
| 其它 | 5 | 7 |

**外部引用合计约 175 处，分布在 30 多个文件。**

### 4.1 外部依赖的具体用途

| 文件 | 访问内容 | 为什么需要 |
|---|---|---|
| `ime.rs` (32) | `chat_ui().input_focused`、`.draft`、模型选择器查询、输入框几何 | 聊天输入框的 IME 光标定位与候选 |
| `settings/cards.rs` (19) | `focused_settings_input()`、`owns_settings_input()`、`model_selector_*` | 设置页文本框的输入归属判定 |
| `settings/ai/*` (36) | 提供者、密钥、MCP 服务器增删改 | AI 设置界面本身 |
| `actions.rs` (11) | 确认对话框动作、退出拦截、搜索 | 全局快捷键与脏状态守卫 |
| `root/render.rs` (9) | 悬浮聊天层、模型选择器浮层、内联提示面板 | 面板级渲染 |
| `root/modal_owner.rs` (5) | 聊天确认框的模态所有权与按键路由 | 模态栈 |
| `pane_tree.rs` (5) | 上下文选择、截图 | 面板 → 模型的上下文注入 |
| `remote_desktop/session.rs` (4) | 远程桌面截图 | 把画面喂给模型 |
| `connection_monitor/health/*` (8) | 主机信息查询 | 监控结果可被模型读取 |
| `history_quit.rs` (4) | 有草稿时拦截退出 | 防丢数据 |

### 4.2 已完成的第一步解耦

commit `83eeb2b9f` 把**上下文侧栏的宽度与拖拽状态**从 `AiChatWorkspaceState` 迁到了 `WorkspaceApp`：

```rust
// WorkspaceApp 新增
context_sidebar_width: f32,
context_sidebar_resizing: bool,
```

原因：侧栏是 workspace 级 UI，它的宽度不该寄生在 AI 对话状态里。这个改动本身是正确的架构改进，也让 `sidebar/state.rs` 和 `sidebar/region.rs` 彻底脱离 `ai_entity`。

**这说明解耦是可行的，只是要一个入口一个入口地做。**

## 5. 删除 AI 的难点

按上面的耦合地图，真正的难点不在 `sidebar/ai`（560 处但都在自己目录内，整体删掉即可），而在 30 多个外部文件的 178 处引用，它们分属四类：

1. **IME 命中测试**（`ime.rs` 32 处）——`WorkspaceImeTarget` 有 4 个 AI 变体（`AiChatInput` / `AiMessageEdit` / `AiInlinePrompt` / `AiConversationRename`），每个都有独立的换行、行高、水平内边距、光标定位分支
2. **设置页输入归属**（`settings/cards.rs` 19 处）——`owns_settings_input()` 决定一个 `SettingsInput` 由设置实体还是 AI 实体处理
3. **模态与脏状态守卫**（`root/modal_owner.rs`、`actions.rs`、`history_quit.rs`）——退出/切换时的拦截条件
4. **上下文注入**（`pane_tree.rs`、`connection_monitor/`、`remote_desktop/`）——把外部状态喂给模型

前三类删除后行为明确（不再有 AI 输入框、不再有 AI 模态）；第四类需要确认删除后对应的功能入口一并消失。
