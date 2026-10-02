# 集成边界：i18n、持久化、发布

> 对应 v2.2.8（`83eeb2b9f`）。

## 1. 国际化

### 1.1 装配方式

2 个语言（简体中文、英文）× 18 个领域文件 = 36 个 JSON，在编译期用 `include_str!` 打进二进制（`crates/oxideterm-i18n/src/lib.rs:8` 起，每个语言一个 `&[&str]` 数组）。

这意味着**切换语言不需要读磁盘、不需要网络**，代价是二进制里固定带 36 份文案。

### 1.2 查找与回退

```rust
pub fn t(&self, key: &str) -> String {                    // :276
    self.catalog_message(self.locale, key)
        .or_else(|| self.catalog_message(self.fallback_locale, key))  // 英文
        .unwrap_or_else(|| key.to_string())                            // 都没有 → 显示 key 本身
}
```

三级回退：当前语言 → 英文 → 显示键名。**最后一级是刻意设计**——漏翻译会立刻在界面上暴露成 `"settings_view.help.xxx"` 这样的裸键，而不是静默显示空白。

### 1.3 装配时机

`ensure_catalog`（`:284`）按需解析 JSON 并缓存。语言收窄到 2 个之后，`I18n::new` 预载的「当前语言 + 英文回退」已经覆盖全部语言表，所以懒加载不再是有意义的优化，机制保留但恒定命中缓存。

### 1.4 一致性保障

两条防线：

| 机制 | 位置 | 检查什么 |
|---|---|---|
| Rust 单测 | `locale_catalogs_have_the_same_complete_key_set` | 2 个语言必须有**完全相同**的键集合 |
| Python 审计 | `scripts/quality/audit_i18n.py` | 源码引用的键是否都存在、占位符是否一致、非英语言是否还是英文原文 |

第二条会输出 "English-copy warnings"——提醒某语言的文案其实没翻译。这是有意的质量提示，不是错误。

## 2. 持久化

### 2.1 设置文件

`PersistedSettings`（`crates/oxideterm-settings/src/model/misc.rs:441`）整体 `#[serde(rename_all = "camelCase")]`。

**已知不一致**：`GeneralSettings` 没有加 `rename_all`，所以它的字段序列化成 snake_case（如 `update_channel`），而其它段是 camelCase。源码按页面拆成 `model/{base,highlight,terminal,ui_connection,ai,misc}.rs`，用 `include!`（`model.rs:10-16`）合并进一个模块——目的是让 serde 字段名和公共导出位置保持稳定，同时文件按设置页组织。

### 2.2 加载时清洗

`normalize.rs` 是加载管线：枚举白名单、数值区间钳制、缺失字段补默认值、收集 `validation_warnings`。

清洗在**反序列化之后**进行，所以 `#[serde(default)]` 负责结构完整，`normalize` 负责值域合法。两者职责不重叠。

### 2.3 数据目录

`default_settings_path()`（`store.rs:117`）三级优先：

1. 便携模式（`portable_data_dir()` 命中）→ 可执行文件旁的 `data/`
2. 启动时指定的目录（`bootstrap_data_dir()`）
3. 系统用户数据目录

### 2.4 敏感数据边界

| 数据 | 落地形式 | 保护 |
|---|---|---|
| 连接密码 / 私钥口令 | 只存 `keychain_id` | 系统钥匙串，明文字段 `skip_serializing` |
| 供应商 API key | 同上 | `oxideterm-ai/src/key_store.rs` |
| 提权凭据 | 同上 | `store/helpers.rs` |
| 便携模式下的上述全部 | 口令加密 keystore | `portable-runtime/src/keystore.rs` |
| 会话树快照 | 普通 JSON | 只含连接描述，不含凭据 |
| `.oxide` 导出 | 加密归档 | `oxide_file/format.rs`，每种载荷有手写 `Debug` 防泄漏 |

## 3. 发布打包

`scripts/release/package_native.py` 是唯一的发布入口。

### 3.1 组成

| 组件 | 定义 | 说明 |
|---|---|---|
| 主程序 | `APP_BIN = "oxideterm-native"` (`:43`) | |
| 远程桌面 helper | `HELPER_BINS = ("oxideterm-rdp-helper", "oxideterm-vnc-helper")` (`:46`) | 打进 `Resources/helpers/<target>/` |
| CLI 助手 | `Resources/cli-bin/` | 用户在设置里一键安装 |
| 许可文件 | `RELEASE_DOCUMENTS` (`:69`) | 源码许可证 + 第三方声明 |
| 便携数据目录 | `PORTABLE_DATA_DIR = "data"` (`:51`) | 首次运行创建 |

### 3.2 macOS 流程

```
build app + helpers
  → 组装 OxideTerm.app（Info.plist、图标、entitlements）
  → sign_macos_path()                              签名
  → ditto -c -k → app.zip                           公证提交容器
  → notarize_macos_artifact()                       Apple 公证
  → xcrun stapler                                   装订票据
  → hdiutil → .dmg
  → create_portable_package() → portable.tar.gz
```

签名身份从 `MACOS_CODESIGN_IDENTITY` 读（`:607`），默认 `-`（ad-hoc）。`should_build_macos_notarization_zip()` 为 false 时跳过 zip——ad-hoc 构建没有可公证的东西。

`.app.zip` 只在有真实签名身份时生成；`.app.tar.gz`（1.x updater 桥接）已在本轮移除。

### 3.3 其它平台

- **Windows**：NSIS 脚本生成 `.exe` 安装包（`stage_windows_installer_root`）
- **Linux**：AppImage、deb、rpm 三种（`create_linux_appimage` / `create_linux_deb` / `create_linux_rpm`），带图形库依赖推荐（`libegl1`、`libvulkan1`）
- **portable**：三平台通用的 tar.gz

## 4. 体积与构建配置

### 4.1 release profile

`Cargo.toml` 的 `[profile.release]`：

```toml
lto = "fat"
codegen-units = 1
strip = true
panic = "abort"
opt-level = 3
```

`opt-level = 3` 是刻意选择——profile 里有注释说明：降到 `"s"` 会牺牲终端渲染吞吐。

### 4.2 二进制段构成（实测）

主二进制 76.7 MB：

| 段 | 大小 | 内容 |
|---|---|---|
| `__text` | 51.3 MB | 编译后的代码 |
| `__const` | 20.9 MB | 只读数据（字体、ICU、tree-sitter 表） |

**压缩特征很关键**：`__const` 里的数据压缩率高达 7.8 倍（tree-sitter 解析表是 16 位整数数组，极度重复），而整体包的压缩率只有 2.2–2.9 倍。删代码（`__text`）对 DMG 的边际收益远高于删数据（`__const`）。

### 4.3 依赖体积大户

| 依赖 | 未压缩体积 | 压缩率 | 可替代性 |
|---|---|---|---|
| 35 个 tree-sitter 语法 | 51.4 MB | ~7.8x | 只需 4 个 shell 语法（7.6 MB） |
| rav1e + ravif（AVIF） | 37.4 MB | 中 | 无调用方，可关 feature |
| exr（OpenEXR） | 18.7 MB | 中 | 无调用方，可关 feature |
| ttf-parser + rustybuzz | 21.5 MB | 低 | 文本整形，Markdown 渲染需要 |
| usvg + resvg + tiny-skia | 14.7 MB | 低 | SVG 栅格化，Markdown 需要 |
| icu 数据 | 4.3 MB | 高 | Unicode 规范化 |

## 5. 质量门禁

| 检查 | 位置 |
|---|---|
| i18n 键集合一致 | `locale_catalogs_have_the_same_complete_key_set` 单测 |
| i18n 引用完整性 + 占位符 | `scripts/quality/audit_i18n.py` |
| 打包产物校验 | `scripts/release/verify_native_package.py` |
| 打包脚本自身 | `scripts/tests/test_package_native.py`（63 个测试） |
| 格式 | `cargo fmt --all -- --check`（注意：仓库配置用了 nightly-only 选项，stable 下会有差异） |
