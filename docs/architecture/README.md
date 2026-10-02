# OxideTerm 架构文档

> 基于 v2.2.8（commit `83eeb2b9f`）静态梳理。规模：68 crate / 867,278 行 Rust。
> 所有结构性论断都标注了 `路径:行号`，可回溯核对。

## 阅读顺序

| 文档 | 内容 | 适合谁 |
|---|---|---|
| [OVERVIEW.md](OVERVIEW.md) | crate 分层、依赖图、主程序内部结构、核心数据模型 | 第一次接触本项目 |
| [RUNTIME.md](RUNTIME.md) | 实体树与所有权、终端会话抽象、事件投递、SSH 连接所有权、线程模型 | 要动核心运行时 |
| [AI.md](AI.md) | OxideSens 四层结构、句柄签发机制、**耦合地图** | 要删除或改造 AI |
| [REMOTE.md](REMOTE.md) | SSH/SFTP/远程桌面/转发/连接存储/加密归档 | 要动远端能力 |
| [INTEGRATION.md](INTEGRATION.md) | i18n、持久化、凭据边界、发布打包、体积构成 | 要动集成层或打包 |
| [REMOVE-AI-PLAN.md](REMOVE-AI-PLAN.md) | 分 7 批删除 AI 助手的执行计划、风险与回滚 | 要做 AI 删除 |

## 三十秒版

**这是一个 Rust 原生桌面 SSH 客户端**，自 vendored 的 GPUI 做渲染、vendored 的 alacritty-terminal 做终端模拟，没有 WebView。

- **根实体**是 `WorkspaceApp`（`workspace.rs:695`），所有状态挂它或它的子实体
- **窗口壳层不做业务渲染**，只管原生窗口样式，把内容委托给共享的 `WorkspaceApp`——主窗口和分离窗口因此共享状态
- **SSH 连接的所有权在 `NodeRouter`**，不在任何单个面板。终端、SFTP、转发、远程桌面对同一主机共享一条连接
- **终端有五种后端**（本地 PTY / SSH / Telnet / Mosh / 串口），统一到 `TerminalSessionBackend` trait
- **凭据在序列化层就被挡住**：明文字段带 `skip_serializing`，磁盘上只有钥匙串引用
- **AI 助手是纯逻辑 + GPUI 壳的分离结构**：`oxideterm-ai` 零 GPUI 依赖，可完整单测

## 已知的架构债

按影响面排序，供后续决策参考：

| 问题 | 位置 | 影响 |
|---|---|---|
| `AiWorkspaceEntity` 扩散到 30+ 文件 | 见 [AI.md §4](AI.md#4-耦合地图重要) | 删除 AI 面板需要跨 10 个目录分批改 |
| `GeneralSettings` 序列化不一致（snake_case vs camelCase） | `model/misc.rs:441` | 迁移脚本要特殊处理 |
| 198 份文案打进二进制 | `i18n/src/lib.rs:13` | 二进制固定增大，运行时无法按需下载语言包 |
| `__const` 里的 tree-sitter 表压缩率 7.8x | 依赖树 | 未压缩体积远大于分发体积，评估收益时容易误判 |
| 仓库 rustfmt 配置含 nightly-only 选项 | `rustfmt.toml` | stable 下 `cargo fmt --check` 必然报差异 |

## 本轮相关变更

文档梳理基于以下 commit（本次会话的体积优化与功能删除）：

```
83eeb2b9f  侧边栏宽度状态迁出 AI 实体
88dffc1f8  补回 A1 漏掉的 mosh options 重导出
2fb4eace7  删除 Public MCP 桥接             (-11,251 行)
ee51b93bc  删除知识库文档工作区            (-7,465 行)
6de9ee323  语法瘦身 35 → 4               (-2,573 行, __const -38 MB)
5dce68c34  删除 SFTP 预览/编辑器/diff     (-4,608 行)
449ab55be  六项功能删除                   (-19,887 行)
```

净结果：主二进制 122.1 → 76.7 MB（−37.2%），DMG 51.2 → 45.2 MB（−11.7%）。
