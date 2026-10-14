# QuickHosts

基于 Rust + egui 开发的轻量、极速跨平台 Hosts 管理与切换工具。

## 特性

- **多方案管理**：支持多个本地 Hosts 方案与组合管理。
- **远程订阅**：支持远程 Hosts 源订阅与定时自动拉取更新。
- **实时预览**：提供激活方案合并预览与语法高亮查看。
- **系统集成**：Linux 下自动集成桌面图标及应用菜单 (`.desktop`)。
- **极小资源占用**：原生二进制构建，启动迅速，内存占用极低。

---

## 依赖要求 (Linux)

构建前需安装 Rust 工具链及系统图形基础库：

```bash
# Ubuntu / Debian
sudo apt-get install -y build-essential pkg-config libx11-dev libxcursor-dev libxrandr-dev libxi-dev libwayland-dev libxkbcommon-dev
```

---

## 开发与编译

```bash
# 开发运行
cargo run

# Release 优化编译
cargo build --release
```

编译产物位于 `target/release/quickhosts`。

---

## 打包指南 (Debian / Ubuntu)

本项目支持通过 `cargo-deb` 制作 `.deb` 安装包。

### 1. 安装打包工具

```bash
cargo install cargo-deb
```

### 2. 标准打包 (x86_64)

```bash
cargo deb
```
产物位置：`target/debian/io.github.geekdex.quickhosts_<version>_amd64.deb`（体积约 4MB）。

### 3. 极限压缩打包

利用系统 `xz -9e` 进行更强力压缩以缩减安装包体积：

```bash
XZ_OPT="-9e" cargo deb --compress-system
```

### 4. 跨架构打包 (以 ARM64 为例)

`.deb` 包为单架构规范，如需生成对应架构的独立安装包：

```bash
# 添加目标架构
rustup target add aarch64-unknown-linux-gnu

# 构建并打包 ARM64 版本
cargo deb --target aarch64-unknown-linux-gnu
```

---

## 权限说明

修改系统 Hosts 文件需要管理员权限：
- Linux / macOS：若保存时提示无权限，程序会引导或需要以 `sudo` 运行。
- Windows：请右键选择“以管理员身份运行”。

## 数据存储与卸载清理 (Data Storage & Uninstall)

QuickHosts 的运行时配置及自定义 Hosts 方案均保存在用户配置目录中，不会分散污染系统其他路径。

### 运行时数据目录

各操作系统的默认存储路径如下：

| 操作系统 | 数据存储路径 | 快捷访问说明 |
| :--- | :--- | :--- |
| **macOS** | `~/Library/Application Support/quickhosts/` | 在访达 (Finder) 按 `Cmd + Shift + G` 输入前往 |
| **Windows** | `%APPDATA%\quickhosts\`<br>（完整路径：`C:\Users\<用户名>\AppData\Roaming\quickhosts\`） | 在资源管理器地址栏输入 `%APPDATA%\quickhosts` 并回车 |
| **Linux** | `~/.config/quickhosts/`<br>（遵循 XDG 规范，即 `$XDG_CONFIG_HOME/quickhosts`） | 终端路径或文件管理器显示隐藏文件 (`Ctrl + H`) 查看 |

#### 目录内文件说明：
- `config.json`：核心数据文件，存储所有自定义 Hosts 方案、远程订阅源、开关状态、回收站及界面偏好设置。
- `hosts.backup`：首次启动时自动备份的系统原始 Hosts 文件。
- `quickhosts.sock`：运行时的单实例通信套接字（退出时自动清理）。

---

### 卸载干净清理步骤

如果您需要彻底卸载 QuickHosts 并清理所有残留数据，请按照以下步骤操作：

#### 1. 清理系统 Hosts 注入规则
- **推荐方法（卸载前）**：在打开 QuickHosts 时，直接关闭顶部“全局总开关”，系统 Hosts 中的 QuickHosts 托管内容将被自动干净移除。
- **手动清理（已卸载后）**：打开系统 Hosts 文件（macOS / Linux 为 `/etc/hosts`，Windows 为 `C:\Windows\System32\drivers\etc\hosts`），删除由 QuickHosts 生成的标记区块即可：
  ```hosts
  # --- QuickHosts START (Auto-Generated) ---
  ...（中间为 QuickHosts 生成的内容）...
  # --- QuickHosts END ---
  ```
  *(注：如果需要，也可使用数据目录中的 `hosts.backup` 恢复原始系统 hosts)*

#### 2. 删除数据目录
终端执行对应命令删除数据目录：
- **macOS**：
  ```bash
  rm -rf ~/Library/Application\ Support/quickhosts
  ```
- **Linux**：
  ```bash
  rm -rf ~/.config/quickhosts
  ```
- **Windows (PowerShell / CMD)**：
  ```cmd
  rmdir /s /q "%APPDATA%\quickhosts"
  ```

---

## 开源协议 (License)

本项目遵循以下双重开源协议，用户可二选其一（Dual-licensed）：

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)
