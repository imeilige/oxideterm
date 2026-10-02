# OxideTerm 架构总览

> 本文基于 v2.2.8（commit `83eeb2b9f`）的代码静态梳理。所有结构性论断都标注了 `路径:行号`，可回溯核对。
> 规模数据：68 个 crate，867,278 行 Rust。

## 1. 项目定位

OxideTerm 是一个 Rust 原生的桌面 SSH / 终端客户端。它不是"WebView 套壳"——渲染层是自带的 GPUI（vendored 在 `crates/gpui-ce`），终端模拟器是 vendored 的 `alacritty-terminal` + `vte`，没有浏览器运行时。

一次会话里可能同时存在：多个终端面板、本地 shell、远程 shell、SFTP 双栏文件浏览、端口转发、远程桌面（RDP/VNC 子进程）、会话树、连接监控、以及一个可开关的 AI 助手面板。

## 2. crate 分层

依赖是严格单向的，没有环。按"被依赖次数"排出的核心层：

| crate | 被几个 crate 依赖 | 职责 |
|---|---|---|
| `oxideterm-settings` | 9 | 持久化设置模型 + 落盘 + 加载时清洗 |
| `oxideterm-gpui-ui` | 7 | 通用 UI 组件（按钮、输入框、列表、动效） |
| `oxideterm-connections` | 7 | 保存的连接、凭据、导入导出 |
| `oxideterm-ssh` | 7 | SSH 协议栈、连接注册表、`NodeRouter` |
| `oxideterm-forwarding` | 4 | 端口转发管理 |
| `oxideterm-terminal` | 3 | 终端会话内核（解析器、网格、滚动） |
| `oxideterm-sftp` | 3 | SFTP/SCP/归档传输 |
| `oxideterm-settings-model` | 3 | 设置的 UI 身份模型（tab/select/input 枚举） |

叶子层（无人依赖，直接进二进制）：`oxideterm-gpui-app`（主程序）、`oxideterm-cli`、`oxideterm-rdp-helper`、`oxideterm-vnc-helper`、`gpui-ce`、`alacritty-terminal`、`vte`、`fernomade`。

### 2.1 代码量分布

```
270,936  oxideterm-gpui-app      主程序，占全仓 31%
 48,961  oxideterm-ai            AI 核心逻辑
 33,924  oxideterm-gpui-terminal 终端面板 UI
 28,784  oxideterm-terminal      终端模拟器封装
 24,996  oxideterm-connections   连接存储
 21,565  oxideterm-gpui-ui       通用组件
 19,248  oxideterm-ssh           SSH 协议
  15,772  oxideterm-cli           命令行
 15,186  oxideterm-connection-monitor  连接监控
```

## 3. 主程序的内部结构

`crates/oxideterm-gpui-app/src/workspace/` 是应用主体（`workspace.rs:695` 定义 `WorkspaceApp` 根实体）。它内部再按功能切分：

| 子目录 | 行数 | 内容 |
|---|---|---|
| `sidebar/` | 42,282 | 左侧主边栏 + AI 上下文侧栏（`sidebar/ai/` 36,141 行） |
| `settings/` | 23,778 | 设置界面 |
| `new_connection/` | 23,775 | 新建连接向导（SSH 主机密钥、密码、交互式认证） |
| `connection_monitor/` | 22,451 | 连接健康、主机工具采样（进程/端口/文件系统/tmux） |
| `session_manager/` | 16,384 | 会话树管理 |
| `sftp/` | 13,331 | SFTP 面板与对话框 |
| `tabs/` | 13,059 | 标签页宿主、面板树、分离窗口 |
| `terminal_command_bar/` | 9,224 | 命令栏补全与发送 |
| `file_manager/` | 7,775 | 本地文件管理 |
| `remote_desktop/` | 6,346 | RDP/VNC 会话 |
| `forwards/` | 6,233 | 端口转发 UI |
| `root/` | 5,936 | 窗口壳层、渲染入口、模态所有权、IME 路由 |
| `ai_state/` | 3,924 | AI 状态实体（+ `ai_state.rs` 6,882 行） |
| `ai_runtime_context/` | 1,109 | AI 工具句柄签发与能力注册 |

根目录文件：`keybindings.rs`（2,027 行，快捷键定义）、`workspace.rs`（1,138 行）、`portable_bootstrap.rs`（835 行，便携模式启动）、`single_instance.rs`（665 行）、`main.rs`（456 行）。

## 4. 核心数据模型

### 4.1 标签页种类

`crates/oxideterm-workspace/src/lib.rs:28` 定义 `TabKind`：

```
Workspace | LocalTerminal | SshTerminal | MoshTerminal | FileManager | Graphics
Sftp | Forwards | SessionManager | RemoteDesktop | Settings
```

### 4.2 SSH 节点路由

`NodeRouter`（`crates/oxideterm-ssh/src/router/node_router.rs:2`）是 SSH 连接的唯一权威所有者：

```rust
pub struct NodeRouter {
    registry: SshConnectionRegistry,   // 实际连接池
    runtime: NodeRuntimeStore,         // 节点就绪状态
    emitter: NodeEventEmitter,         // 事件广播
}
```

设计要点：终端面板、文件管理器、端口转发、远程桌面对同一台远程主机的访问**共享同一个 SSH 连接**。关闭某个消费者（如终端面板）不会断开仍被其他消费者使用的连接——这是 AGENTS.md 里明确的会话所有权规则，由 `NodeRouter` 而非单个面板持有。

### 4.3 持久化设置

`PersistedSettings`（`crates/oxideterm-settings/src/model/misc.rs:441`）是一个大结构体，`#[serde(rename_all = "camelCase")]`：

```
version, general, terminal, buffer, appearance, connection_defaults,
treeUI, sidebarUI, windowUI, settings_navigation, ai, local_terminal,
sftp, ide, reconnect, connection_pool, network, experimental,
onboarding_disclaimer_accepted, onboarding_completed,
command_palette_mru, keybindings, custom_themes, agent_roles,
new_connection, ssh_config, ...
```

源码按页面拆成 `model/{base,highlight,terminal,ui_connection,ai,misc}.rs`，用 `include!` 合并进一个模块（`model.rs:10-16`），这样 serde 字段名和公共导出位置不变，但文件按设置页组织。

## 5. 设置系统的三层拆分

同一套设置被拆进三个 crate，职责不重叠：

| crate | 放什么 | 例子 |
|---|---|---|
| `oxideterm-settings` | 磁盘上的数据 + 清洗 + 落盘 | `PersistedSettings`、`sanitize_settings_value`、`SettingsStore` |
| `oxideterm-settings-model` | UI 身份与纯逻辑（无 GPUI） | `SettingsTab`、`SettingsSelect`、`SettingsInput`、`input_draft.rs`、`navigation.rs` |
| `oxideterm-gpui-settings-view` | 展示层构件 | `settings_appearance_card_shell`、`options.rs`（下拉项构造）、`labels.rs` |

`input_draft.rs` 是关键：它把 `SettingsInput` 枚举映射到"读当前值 / 写回草稿 / 解析成类型"三个操作，UI 层不碰解析逻辑。

加载时会经过 `normalize.rs` 的清洗管线：枚举值白名单、数值区间钳制、缺失字段补默认值。**注意** `GeneralSettings` 没有 `rename_all`，所以它序列化成 snake_case 的 `update_channel`，而其它段是 camelCase——这是历史遗留的不一致。

## 6. 窗口模型

`WindowRegistry<Handle, Effect, CoalescingKey>`（`crates/oxideterm-gpui-app/src/workspace/window_registry.rs:55`）是一个泛型注册表，管理原生窗口与"窗口级效果"的对应关系。

- `WindowRole` 区分主窗口和分离标签窗口
- 效果类型是 `WorkspaceWindowEffect`（窗口意图、运行时事件、AI、公共 MCP、标签宿主、图形）
- `coalescing_key()` 让同类效果在重绘前合并，避免事件风暴
- 注册/释放走 `reserve_workspace_window` → `commit_workspace_window` 两阶段

分离标签页各自持有 `DetachedTabWindow`（`detached_tab_window.rs`），与主窗口共享同一个 `WorkspaceApp` 实体。

## 7. 数据落盘位置

`default_settings_path()`（`crates/oxideterm-settings/src/store.rs:117`）的优先级：

1. 便携模式（`portable_data_dir()` 命中）→ 跟随可执行文件旁的 `data/`
2. 启动时指定的数据目录
3. 系统用户数据目录

密钥单独走 `oxideterm-secret-store`（macOS 用 Keychain，Windows/Linux 有对应实现），普通设置文件只存非敏感元数据。
