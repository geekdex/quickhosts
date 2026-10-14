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

---

## 开源协议 (License)

本项目遵循以下双重开源协议，用户可二选其一（Dual-licensed）：

- [Apache License, Version 2.0](file:///home/panhy/src/geekdex/switch_hosts/quickhosts/LICENSE-APACHE)
- [MIT License](file:///home/panhy/src/geekdex/switch_hosts/quickhosts/LICENSE-MIT)
