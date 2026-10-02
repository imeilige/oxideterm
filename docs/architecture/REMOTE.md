# 远端能力

> 对应 v2.2.8（`83eeb2b9f`）。

## 1. SSH 连接层

### 1.1 `NodeRouter` 与连接池

`crates/oxideterm-ssh/src/router/node_router.rs:2`：

```rust
pub struct NodeRouter {
    registry: SshConnectionRegistry,   // 实际连接池
    runtime: NodeRuntimeStore,         // 节点就绪状态
    emitter: NodeEventEmitter,         // 事件广播
}
```

`SshConnectionRegistry` 维护到各远程主机的活跃连接。`NodeRouter` 在构造时把 emitter 注入 registry（`:20`），registry 因此不直接发事件。

**共享语义**：终端、SFTP、转发、远程桌面对同一主机共用一条 SSH 连接。每个消费者持有自己的 lease，最后一个释放时才真正断开。

### 1.2 crate 内部结构

```
crates/oxideterm-ssh/src/
    connection_registry.rs   连接池
    router.rs / router/      NodeRouter
    transport.rs / transport/  传输层
    host_key.rs              主机密钥校验
    reconnect.rs             重连策略
    session_tree_plan.rs     会话树规划
    capability.rs            能力协商
    agent_endpoint.rs        agent 端点
    upstream_proxy.rs        上游代理
    monitor.rs               连接监控
```

## 2. SFTP 与文件传输

### 2.1 crate 结构

```
crates/oxideterm-sftp/src/
    session.rs / session/     SFTP 会话
    scp.rs / scp/             SCP 兼容通道
    transfer_manager.rs       传输队列
    archive.rs                归档
    tar_transfer.rs           tar 流式传输
    conflict.rs               冲突消解
    retry.rs                  重试
    path_utils.rs             路径规范化
    text_diff.rs              文本对比
```

### 2.2 传输语义

`TransferDirection`（`types.rs:85`）只有上传/下载两向。下载目标的行为由 `LocalDownloadDisposition`（`types.rs:91`）显式控制：

```rust
pub enum LocalDownloadDisposition {
    CreateNew,        // 目标已存在则失败
    ReplaceExisting,  // 需要调用方明确给出覆盖决定
    ResumeVerified,   // 只在传输记录校验通过后续传
}
```

这是安全默认值：不会静默覆盖用户的文件，也不会在未校验的情况下续传。

## 3. 远程桌面

### 3.1 provider 机制

`crates/oxideterm-remote-desktop/src/provider.rs:58` 定义 manifest：

```rust
pub struct RemoteDesktopProviderManifest {
    id, name, description, version,
    protocol: RemoteDesktopProtocol,
    entry: RemoteDesktopProviderEntry,     // command + args + workingDir
    capabilities: RemoteDesktopProviderCapabilities,
    ui: Option<RemoteDesktopProviderUi>,   // 默认端口、图标等
}
```

`validate()`（`:72`）检查 id 非空、name/version 非空、`entry.command` 非空。`validate_provider_id`（`:259`）额外拒绝含路径分隔符和 `..` 的 id——防止通过 provider id 做路径穿越。

内置两个 provider（`builtin_provider_manifest`，`:190` 附近）：

| 协议 | 命令 |
|---|---|
| RDP | `oxideterm-rdp-helper` |
| VNC | `oxideterm-vnc-helper` |

两者都是独立二进制（9,103 / 12,410 行），通过 stdio 与主程序通信（`--stdio`），由 `helper_process.rs` 管理生命周期。

### 3.2 数据通路

```
provider 子进程 (stdio)
  → helper_protocol.rs  解码
  → frame_queue.rs      帧队列
  → worker.rs           解码/重绘节流
  → GPUI 纹理
```

证书校验独立在 `certificate_store.rs`，首次连接需要用户确认并记录指纹。

## 4. 端口转发

`crates/oxideterm-forwarding/src/`：

| 文件 | 职责 |
|---|---|
| `manager.rs` | `ForwardingManager`（`:26`）总控 |
| `model.rs` | 规则模型 |
| `local.rs` | 本地监听 |
| `dynamic.rs` | 动态端口分配 |
| `bridge.rs` | 与 SSH 通道桥接 |
| `profiler.rs` | 流量统计 |
| `detection.rs` | 端口冲突检测 |
| `events.rs` | 事件 |

## 5. 连接存储

### 5.1 `SavedConnection`

`crates/oxideterm-connections/src/store/types.rs:576`：

```rust
pub struct SavedConnection {
    id, version, name,
    group: Option<String>,      // 分组
    notes: Option<String>,      // 自由元数据（UI 明确警告不要存凭据）
    host, port, username, …
}
```

`ConnectionStore`（`connection_store.rs`）提供 `connections()`（`:59`）、`managed_ssh_keys()`（`:3038`）等只读访问。

### 5.2 凭据模型

`SavedAuth`（`types.rs:34`）的每个变体都遵循同一模式：

```rust
Password {
    empty_password: bool,
    keychain_id: Option<String>,                              // 钥匙串里的引用
    #[serde(rename = "password", skip_serializing)]            // 明文绝不落盘
    plaintext_password: Option<SecretString>,
}
```

`skip_serializing` 是硬保证：即使明文还在内存里，序列化到磁盘也会被跳过。磁盘上永远只有 `keychain_id`。

变体包括 `Password`、`Key`（私钥 + 口令）、以及 Agent/无认证等。

## 6. `.oxide` 加密归档

`crates/oxideterm-connections/src/oxide_file/` 实现了带版本头和加密载荷的导入导出格式。

**文件头**（`format.rs:29`）：

```rust
pub struct …Header {
    magic: [u8; 5],
    version: u32,
    kdf_version: u32,
}
```

**加密载荷类型**（每种都有手写的 `Debug` 实现，防止密钥材料意外打印）：

| 类型 | 行号 |
|---|---|
| `EncryptedPayload` | `format.rs:133` |
| `EncryptedPluginSetting` | `format.rs:195` |
| `EncryptedPortableSecret` | `format.rs:211` |
| `EncryptedConnection` | `format.rs:228` |
| `EncryptedPrivilegeCredential` | `format.rs:292` |
| `EncryptedUpstreamProxyPolicy` | — |
| `EncryptedUpstreamProxyAuth` | — |

导出/导入的往返计数在 `transfer/import.rs:288` 起，用于给用户报告"恢复了 N 个连接 / N 个凭据"。

## 7. 密钥存储抽象

`crates/oxideterm-secret-store/`：

| 文件 | 平台 |
|---|---|
| `macos.rs` | macOS Keychain |
| `macos_auth.rs` | macOS 授权流程 |
| `lib.rs` | 跨平台 trait |

便携模式下改用 `oxideterm-portable-runtime/src/keystore.rs` 的口令加密 keystore。`ai` crate 的供应商密钥（`key_store.rs`）、连接的认证凭据（`keychain.rs`）、提权凭据（`store/helpers.rs`）都走同一套抽象。
