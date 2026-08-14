<div align="center">

# 空闲盒 (IdleBox)

**告别 Busy，拥抱 Idle。**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![Size](https://img.shields.io/badge/size-~360KB-green.svg)](target/release/idlebox)

[🇬🇧 English](README.md)

</div>

---

## 简介

**空闲盒 (IdleBox)** 是一个独立、轻量、高颜值的 BusyBox/POSIX 兼容工具箱，使用纯 Rust 编写，零外部依赖。

### 设计理念

> 告别 Busy，拥抱 Idle。

BusyBox 承载了嵌入式 Linux 的半壁江山，但它的 C 代码库已走过了二十余年。IdleBox 希望以现代语言 Rust 重新诠释这一经典理念——在保持 POSIX 兼容性的同时，追求更小的体积、更高的安全性、以及更愉悦的终端体验。

---

## 特性

- **零依赖** — 仅使用 Rust 标准库，不引入任何第三方 crate
- **极致精简** — Release 构建约 360KB，适合嵌入式与容器场景
- **POSIX 兼容** — 常见 Unix 工具的原生替代
- **模块化设计** — 通过 Applet 机制轻松扩展
- **符号链接支持** — 通过符号链接直接调用各 Applet
- **高颜值终端** — 内置 ANSI 彩色输出，让命令行赏心悦目

---

## 已实现的 Applet

| Applet | 说明 | 亮点 |
|--------|------|------|
| `echo` | 输出文本到标准输出 | 支持 `-n` 不换行、`-e` 转义解释 |
| `cat` | 连接文件并输出到标准输出 | 支持 `-n` 行号、`-b` 非空行号、`-A` 显示不可见字符、stdin 管道 |
| `ls` | 列出目录内容 | **ANSI 炫彩输出**：目录蓝色、可执行文件绿色、压缩包红色、链接青色；支持 `-l` 长格式、`-a` 隐藏文件、`-h` 人类可读大小 |
| `mkdir` | 创建目录 | 支持 `-p` 嵌套创建、一次创建多个目录 |
| `rm` | 删除文件或目录 | 支持 `-r` 递归、`-f` 强制、组合 `-rf` |
| `cp` | 复制文件与目录 | 支持 `-r` 递归、`-f` 强制、多源复制到目标目录 |
| `mv` | 移动（重命名）文件与目录 | 原子重命名，自动处理跨设备降级（复制 + 删除） |
| `touch` | 创建空文件或更新时间戳 | 创建新文件、更新已有文件的 mtime/atime |
| `head` | 输出文件的开头部分 | `-n` 行数、`-c` 字节数、多文件标头、stdin 管道 |
| `tail` | 输出文件的末尾部分 | `-n` 行数、`-c` 字节数、环形缓冲高效读取、stdin 管道 |
| `grep` | 在文件或 stdin 中搜索模式 | `-i` 忽略大小写、`-v` 反向匹配、`-n` 行号、`-c` 计数 |
| `chmod` | 修改文件权限位 | 八进制数字模式、`-R` 递归目录遍历 |
| `df` | 报告文件系统磁盘空间使用情况 | 解析 `/proc/mounts` + `statvfs` 系统调用、`-h` 人类可读、按路径查询 |
| `du` | 估算文件空间占用 | `-h` 人类可读、`-s` 汇总、`-d` 深度控制 |
| `relax` | IdleBox 特色：休息一下 | 独特的放松体验，体现 "Idle" 精神 |
| `--install` | 通过符号链接自动部署 Applet | 在目标目录为所有 Applet 创建符号链接；默认安装到 `/usr/local/bin` |

---

## 快速开始

### 构建

```bash
# Debug 构建
cargo build

# Release 构建（极致优化体积）
cargo build --release

# 查看二进制大小
ls -lh target/release/idlebox
```

### 运行

```bash
# 直接调用
idlebox echo "Hello, IdleBox!"
idlebox cat -n README.md
idlebox ls --color=auto -lah

# 自动安装（为所有 Applet 创建符号链接）
idlebox --install              # 安装到 /usr/local/bin
idlebox --install /tmp/bin     # 安装到自定义目录

# 通过符号链接
cd target/release
ln -s idlebox echo
ln -s idlebox ls
./echo "Hello via symlink!"
./ls --color=auto
```

### 测试

```bash
cargo test
```

---

## 添加新 Applet

1. 在 `src/applets/` 下创建新文件
2. 实现 `Applet` trait
3. 在 `src/applets/mod.rs` 中导出
4. 在 `src/core/dispatcher.rs` 中注册

---

## 架构文档

详细的架构设计文档已迁移至独立的文档仓库，以保持主仓库代码的极简与纯粹。

> 📖 **查看架构文档**: [IdleBox Docs](https://github.com/IamKenae/idlebox-docs)

---

## 许可证

[Apache-2.0](LICENSE)

Copyright (c) IdleBox Contributors.
