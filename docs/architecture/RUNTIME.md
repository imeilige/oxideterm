# 核心运行时

> 对应 v2.2.8（`83eeb2b9f`）。所有论断标注 `路径:行号`。

## 1. 实体树与所有权

### 1.1 根实体

`WorkspaceApp`（`crates/oxideterm-gpui-app/src/workspace.rs:695`）是整个应用的根 GPUI 实体。所有功能状态最终挂在这个实体上，或挂在它持有的子实体上。

主窗口的 `Render` 实现（`window_shell.rs`）每帧做三件事：

```rust
self.native_style.apply(&self.session, window, cx);   // 应用 vibrancy / 窗口透明度
let content = self.session.update(cx, |session, cx| {
    session.render_main_window(window, cx)            // 委托给 WorkspaceApp 渲染
});
render_resizable_window_content(content, window)
```

关键点：**窗口壳层不做业务渲染**，它只管原生窗口样式和可调整边框，把内容完全委托给共享的 `WorkspaceApp`。这让主窗口和分离窗口复用同一份状态。

### 1.2 子实体一览

| 实体 | 定义位置 | 职责 | 生命周期归属 |
|---|---|---|---|
| `WorkspaceApp` | `workspace.rs:695` | 根状态 + 全部功能协调 | 应用进程 |
| `WorkspaceWindowTabState` | `workspace.rs:623` | 主窗口标签状态 | 主窗口 |
| `Tabs`（`WorkspaceTabHostEntity`） | `tabs/entity.rs` | 标签宿主、面板树 | 共享 |
| `TerminalPane` | `oxideterm-gpui-terminal/src/app.rs:404` | 单个终端面板 | 面板树叶子 |
| `SettingsWorkspaceEntity` | `settings/entity.rs` | 设置界面的路由与异步操作 | 设置页打开期间 |
| `AiWorkspaceEntity` | `ai_state.rs` | AI 助手全部状态 | 应用进程 |
| `NodeRouter` | `oxideterm-ssh/src/router/node_router.rs:2` | SSH 连接所有权 | 应用进程 |
| `WindowRegistry` | `window_registry.rs:55` | 原生窗口注册与效果投递 | 应用进程 |

### 1.3 面板树

`PaneNode`（`crates/oxideterm-workspace/src/lib.rs:126`）是递归的分割树：

```rust
pub enum PaneNode {
    Page   { pane_id, tab_id },                              // 非终端页（SFTP/设置/…）
    Leaf   { pane_id, session_id: TerminalSessionId },      // 终端叶子
    Group  { id, direction: SplitDirection, children },     // 水平/垂直分割
}
```

`contains_pane`（`lib.rs:198`）递归查找——这是"某个终端会话属于哪个面板"这一问题的答案来源，也是 AGENTS.md 里"不要靠查找第一个关联面板来解析作用域"那条规则的技术落点。

## 2. 终端会话抽象

### 2.1 五种后端统一接口

`crates/oxideterm-terminal/src/session/types.rs:111` 定义 `TerminalSessionBackend` trait：

```rust
pub trait TerminalSessionBackend: Send {
    fn kind(&self) -> TerminalSessionKind;
    fn lifecycle(&self) -> TerminalLifecycle;
    fn process_info(&self) -> TerminalProcessInfo;
    fn read_pending(&mut self) -> bool;
    fn read_pending_with_budget(&mut self, budget: TerminalDrainBudget) -> TerminalDrainReport;
    fn take_events(&mut self) -> Vec<TerminalEvent>;
    fn write_input(&mut self, bytes: &[u8]) -> Result<()>;
    fn write_protocol_bytes(&mut self, bytes: &[u8]) -> Result<()>;  // DSR/鼠标等
    fn activity_receiver(&self) -> TerminalActivityReceiver;
    // …
}
```

`TerminalSessionKind`（`types.rs:2`）有五个变体：`LocalPty | SshPty | Telnet | Mosh | Serial`，对应 `session/` 下的 `local_backend.rs`、`ssh_pty.rs`、`telnet.rs`、`mosh.rs`、`serial.rs`。

这个 trait 是整个终端栈的关键解耦点：上层（面板 UI、搜索、命令栏）完全不知道底下是本地 PTY 还是远程 SSH。

### 2.2 背压与预算

`read_pending_with_budget(&mut self, budget: TerminalDrainBudget)` 显式要求上层给读取预算。这是终端吞吐的保护机制——一次 SSH 通道可能送来几 MB 输出，如果一次性全读会阻塞 UI 帧。

## 3. 事件投递模式

### 3.1 delivery 抽象

`crates/oxideterm-gpui-app/src/workspace/delivery.rs` 定义了一套"后台结果 → UI 线程"的受限投递机制。

```rust
pub struct DeliveryBudget { max_items: usize, max_elapsed: Duration }   // :17

pub struct ActiveDeliveryWake {                                            // :82
    pending: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    notification: Arc<Notify>,        // tokio Notify，只唤醒一个消费者
}
```

发送端 `ActiveDeliverySender<T>::send()`（`:169`）做两件事：写 std mpsc channel，然后 `wake.mark()` 置位并 `notify_one()`。

接收侧 `drain_channel()`（`:185`）循环 `try_recv`，直到**空、断开、或超出预算**三者之一。预算同时按条目数和时间双重限制（`allows_next`，`:40`），避免一次投递饿死渲染。

### 3.2 为什么需要这个

后台 worker（SSH 读流、SFTP 传输、云同步、连接监控采样）产出的事件速率远高于帧率。没有预算机制，UI 线程会被 backlog 卡死。`DeliveryBudget` 把"处理多少"变成显式参数，让每个子系统自己决定节奏。

## 4. SSH 连接所有权

### 4.1 NodeRouter 是唯一权威

```rust
pub struct NodeRouter {                    // oxideterm-ssh/src/router/node_router.rs:2
    registry: SshConnectionRegistry,       // 实际连接池
    runtime: NodeRuntimeStore,             // 节点就绪状态快照
    emitter: NodeEventEmitter,             // 事件广播
}
```

`NodeRouter::with_runtime_store_and_emitter`（`:20`）在构造时把 emitter 注入 registry——**registry 自己不发事件，它把事件交给 NodeRouter 广播**。

这解释了 AGENTS.md 那条规则的技术实现：终端面板、SFTP、转发、远程桌面对同一主机的访问共享 registry 里的同一条 SSH 连接。关闭一个消费者只是释放它自己的 lease，连接在最后一个消费者离开时才断开。

### 4.2 作用域解析

规范的做法是 **活跃终端 → 节点 → 保存的 owner** 三跳解析，而不是从 host 字符串或面板标题反推。这个解析集中在 `NodeRouter` 里，UI 层不允许自己猜。

## 5. 凭据处理

### 5.1 序列化层就挡住明文

`SavedAuth`（`crates/oxideterm-connections/src/store/types.rs:34`）的设计很讲究：

```rust
Password {
    empty_password: bool,
    keychain_id: Option<String>,                                    // 存哪儿
    #[serde(rename = "password", skip_serializing)]                  // 明文永不落盘
    plaintext_password: Option<SecretString>,
}
```

`skip_serializing` 意味着即使内存里有明文（刚输入还没存进钥匙串），序列化到磁盘时这一字段直接被跳过。磁盘上只有 `keychain_id` 这个指针。

`SecretString` 来自 zeroize 家族，析构时清零内存。

### 5.2 三层凭据存储

| 存储 | 用途 | 实现 |
|---|---|---|
| 系统钥匙串 | 密码、私钥口令、代理凭据 | `oxideterm-secret-store`（macOS Keychain / Windows / Linux） |
| 便携 keystore | 便携模式下的同样内容 | `oxideterm-portable-runtime/src/keystore.rs`，口令加密 |
| 连接 JSON | 非敏感元数据 | 普通文件，明文字段被 `skip_serializing` 排除 |

## 6. 渲染路径

```
native window
  → WorkspaceWindowShell::render              (window_shell.rs)
      → native_style.apply()                  应用 vibrancy / 窗口不透明度
      → WorkspaceApp::render_main_window()    (root/render.rs:119)
          → 按 active tab 分派到具体 surface
              → TerminalPane::render         (gpui-terminal)
                  → TerminalSessionBackend::snapshot() 取网格快照
                  → GPUI 元素树绘制
```

## 7. 线程模型

| 位置 | 用途 |
|---|---|
| UI 线程（GPUI） | 所有实体状态变更、`cx.notify()`、渲染 |
| `cx.background_executor()` | 定时器、可移植的阻塞工作（`app.rs:1042`） |
| `cx.spawn(async …)` | 与 UI 实体绑定的异步任务，实体释放时自动取消 |
| `runtime.spawn_blocking()` | CPU 密集工作（语法高亮、diff） |
| tokio `Runtime` | 网络 I/O（SSH、SFTP、云同步） |

铁律：**`cx.spawn` 捕获的实体是弱引用**（`weak.update(...)`），实体释放时任务自然结束，不产生悬挂。终端面板大量使用这个模式做搜索、提权提示、图像解码完成回调（如 `app.rs:1019`、`1712`、`2286`）。
