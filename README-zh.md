<div align="center">

# 空闲盒 (IdleBox)

**告别 Busy，拥抱 Idle。**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![Linux x86_64 ELF 体积](https://img.shields.io/badge/size-~640_KiB-green.svg)](https://github.com/IamKenae/idlebox/actions/workflows/size.yml)

[🇬🇧 English](README.md)

</div>

---

## 简介

**空闲盒 (IdleBox)** 是一个受 BusyBox 启发的独立、轻量、高颜值多调用工具箱，采用 Rust 编写，仅保留少量纯 Rust 依赖，不捆绑 C 库。

### 设计理念

> 告别 Busy，拥抱 Idle。

BusyBox 承载了嵌入式 Linux 的半壁江山。IdleBox 希望以现代语言 Rust 重新诠释它的多调用二进制理念，并逐步提升对 POSIX、BusyBox 和 GNU 常见工作流的兼容能力。

当前阶段首先把 IdleBox 本身做好：在尽量保持灵活、小巧、轻便和高性能的前提下，持续优化项目结构、基础功能与用户体验，再循序推进更广、更深的兼容与替代。这是当前的工程优先级，而不是对项目长期边界的永久限定。

### 当前开发原则

1. **守住轻量基础** — 优先维持单二进制、最小纯 Rust 依赖集、模块化和低运行开销
2. **先优化项目本身** — 首先改善正确性、一致性、基础功能、跨平台体验和可维护性
3. **渐进扩展兼容** — 优先支持日常高频用法，再逐步覆盖 POSIX、BusyBox 和 GNU 的更多行为
4. **以数据决定取舍** — 通过体积、启动时间、吞吐量和测试结果评估功能与抽象的成本

---

## 特性

- **纯 Rust 压缩** — DEFLATE 与 Gzip 使用 `flate2` 的 `miniz_oxide` Rust 后端，不依赖 zlib 或其他 C 压缩库
- **体积优先** — Release 配置针对体积优化；实际大小随目标平台和工具链变化
- **渐进兼容** — 优先覆盖常用 Unix/POSIX 工作流，并逐步扩展 BusyBox/GNU 兼容行为
- **跨平台** — 支持 Linux、macOS 和 Windows
- **模块化设计** — 通过 Applet 机制轻松扩展
- **符号链接支持** — 通过符号链接直接调用各 Applet
- **高颜值终端** — 内置 ANSI 彩色输出，让命令行赏心悦目

---

## 平台支持

| 平台 | 状态 | 说明 |
|------|------|------|
| Linux | 完整支持 | 全部 58 个 Applet |
| macOS | 完整支持 | 全部 58 个 Applet |
| Windows | 部分支持 | 41+ 个 Applet 完整支持；Unix 专属 Applet（chmod, chown, chgrp, id, su）优雅降级 |

---

## 已实现的 Applet

| Applet | 说明 | 亮点 |
|--------|------|------|
| `echo` | 输出文本到标准输出 | 支持 `-n` 不换行，流式写出参数而不再拼接第二份完整字符串 |
| `printf` | 格式化并输出参数 | 重复使用格式串、反斜杠转义、`%s`/`%b`/`%c`、整数与浮点转换、宽度和精度 |
| `true` / `false` | 返回固定退出状态 | 为脚本提供无输出的成功与失败基础命令 |
| `env` | 在修改后的环境中运行命令 | `-i` 空环境、`-u` 删除变量、临时 `NAME=VALUE` 赋值与命令执行 |
| `printenv` | 打印环境变量 | 输出全部或指定变量，可使用 NUL 分隔 |
| `pwd` | 打印当前工作目录 | `-L` 逻辑路径与 `-P` 物理路径解析 |
| `basename` | 去除名称中的目录与后缀 | 可选后缀删除、`-a`/`-s` 多操作数、NUL 分隔输出 |
| `dirname` | 去除名称的最后一个组成部分 | 支持多个操作数与 NUL 分隔输出 |
| `cat` | 连接文件并输出到标准输出 | 支持 `-n` 行号、`-b` 非空行号、`-A` 显示不可见字符、stdin 管道 |
| `ls` | 列出目录内容 | **ANSI 炫彩输出**：目录蓝色、可执行文件绿色、压缩包红色、链接青色；支持 `-l` 长格式、`-a` 隐藏文件、`-h` 人类可读大小 |
| `tree` | 以树状结构列出目录内容 | 连接线布局，`--charset` 切换 UTF-8/ASCII，`-a` 显示隐藏项，`-d` 只列目录，`-L` 限制层级，`-P`/`-I` 可重复且支持 `*`、`?`、`[set]`、`[^set]`、范围与 `|` 分支，`-f` 完整路径前缀，`-i` 无缩进输出，`-F` 类型指示符，`-s`/`-h`/`-p`/`-u`/`-g`/`-D`（UTC）元数据列，`--dirsfirst`/`-r`/`-t` 排序，`-C` 彩色输出，支持 JSON（`-J`）、XML（`-X`）、HTML（`-H`）输出，`--noreport` 省略汇总行，`-o` 先暂存再发布 |
| `mkdir` | 创建目录 | 支持 `-p` 嵌套创建、一次创建多个目录 |
| `rm` | 删除文件或目录 | 支持 `-r` 递归、`-f` 强制、组合 `-rf` |
| `cp` | 复制文件与目录 | 支持 `-r` 递归、`-f` 强制、多源复制到目标目录 |
| `mv` | 移动（重命名）文件与目录 | 原子重命名，自动处理跨设备降级（复制 + 删除） |
| `touch` | 创建空文件或更新时间戳 | 创建新文件、更新已有文件的 mtime/atime |
| `head` | 输出文件的开头部分 | `-n` 行数、`-c` 字节数、多文件标头、stdin 管道 |
| `tail` | 输出文件的末尾部分 | `-n` 行数、`-c` 字节数、环形缓冲高效读取、stdin 管道 |
| `grep` | 在文件或 stdin 中搜索模式 | `-i` 忽略大小写、`-v` 反向匹配、`-n` 行号、`-c` 计数、`-j`/`--threads` 并行多线程搜索 |
| `chmod` | 修改文件权限位 | 八进制数字模式、`-R` 递归目录遍历 |
| `chown` | 修改文件所有者与组 | POSIX `user[:group]` 语法、`-R` 递归、数字 ID 或名称 |
| `chgrp` | 修改组所有权 | 组名或数字 GID、`-R` 递归 |
| `df` | 报告文件系统磁盘空间使用情况 | 解析 `/proc/mounts` + `statvfs` 系统调用、`-h` 人类可读、按路径查询 |
| `du` | 估算文件空间占用 | `-h` 人类可读、`-s` 汇总、`-d` 深度控制 |
| `ps` | 报告当前进程快照 | 解析 `/proc/[pid]/stat` + `cmdline`、`-e`/`-A` 显示所有进程、`-o` 自定义列 |
| `kill` | 向进程发送信号 | POSIX 信号 FFI、支持信号名称（`-TERM`）和编号（`-9`）、`-l` 列出信号 |
| `free` | 显示内存使用情况 | 解析 `/proc/meminfo`、`-h` 人类可读、显示内存与 Swap |
| `uptime` | 显示系统运行时间 | 解析 `/proc/uptime` + `/proc/loadavg`、显示运行时长与 1/5/15 分钟平均负载 |
| `ln` | 创建文件链接 | `-s` 符号链接、`-f` 强制覆盖、默认硬链接、多目标链接到目录 |
| `readlink` | 打印已解析的符号链接 | `-f`/`-e` 规范化为绝对路径、`-n` 不输出末尾换行符 |
| `realpath` | 打印规范化绝对路径 | 规范化已有路径、静默诊断、NUL 分隔输出 |
| `sleep` | 暂停指定时长 | 小数时长、`s`/`m`/`h`/`d` 后缀、多个时长相加 |
| `tee` | 将 stdin 同时复制到文件和 stdout | 多输出文件、`-a` 追加、`-i` 忽略中断，下游关闭后继续写文件 |
| `tar` | 创建、查看与解包归档 | POSIX ustar 标头、目录递归、`-f` 归档、`-z` Gzip 流、`-C` 解包目录与安全路径校验 |
| `gzip` | 压缩或解压 Gzip 流 | 文件与 stdin、`-d`/`-k`/`-f`/`-c`、失败安全输出、纯 Rust DEFLATE |
| `gunzip` | 解压 Gzip 文件 | 与 `gzip -d` 一致的文件命名和 `-k`/`-f`/`-c` 行为 |
| `zcat` | 将 Gzip 数据解压到 stdout | 读取 `.gz` 文件或 stdin，不修改源文件 |
| `unzip` | 查看与解压 ZIP 归档 | Stored 与 Deflate 条目、`-l`、`-o`、`-d`、CRC 校验与 Zip Slip 防护 |
| `md5sum` | 计算和校验 MD5 消息摘要 | `-c` 校验、`-b`/`-t` 二进制/文本、`--status` 静默检查 |
| `sha1sum` | 计算和校验 SHA1 消息摘要 | `-c` 校验、`-b`/`-t` 二进制/文本、`--status` 静默检查 |
| `sha256sum` | 计算和校验 SHA256 消息摘要 | `-c` 校验、`-b`/`-t` 二进制/文本、`--status` 静默检查 |
| `sha512sum` | 计算和校验 SHA512 消息摘要 | `-c` 校验、`-b`/`-t` 二进制/文本、`--status` 静默检查 |
| `b3sum` | 计算和校验 BLAKE3 消息摘要 | 纯自研大文件极致多线程并行分片计算、`-c` 校验 |
| `uname` | 打印系统信息 | POSIX `uname()` FFI、`-a` 全部信息、`-s`/`-n`/`-r`/`-v`/`-m` 单独字段 |
| `test` / `[` | 评估条件表达式 | POSIX 兼容的 `test` 和 `[` 两种形态、文件/字符串/数值测试、逻辑运算符 |
| `expr` | 评估表达式并输出结果 | 算术、比较、逻辑、字符串操作；递归下降解析器 |
| `find` | 在目录层次结构中搜索文件 | 通配符 `-name`、`-type`、`-maxdepth`、`-empty`、`-j`/`--threads` 并行目录遍历器；纯 Rust 遍历 |
| `wc` | 打印换行、单词和字节计数 | 8 KiB 流式计数、`-l`/`-w`/`-c`/`-m`、`-j`/`--threads` 并行计数、多文件 `total`、stdin 管道 |
| `sort` | 排序文本文件的行 | `-r` 反转、`-n` 数值、`-u` 去重、多文件合并 |
| `uniq` | 报告或省略重复行 | 常量级分组内存、可选输出文件、`-c`/`-d`/`-u`/`-i` |
| `cut` | 从每行中移除选定部分 | `-d` 分隔符、`-f` 字段、`-c` 字符、范围支持 |
| `tr` | 转换或删除字符 | SET1/SET2 转换、`-d` 删除、`-s` 压缩、范围扩展 |
| `id` | 打印真实与有效用户及组 ID | `-u`/`-g`/`-G`/`-n` 选项、按用户名查询、POSIX libc FFI |
| `whoami` | 打印有效用户名 | POSIX `geteuid()` + `getpwuid()` FFI |
| `su` | 切换用户 | `-l` 登录 Shell、`-c` 命令、`-s` Shell；仅 root |
| `relax` | IdleBox 特色：休息一下 | 独特的放松体验，体现 "Idle" 精神 |
| `--install` | 自动部署 Applet launcher | 支持 `--dry-run` 预览、默认保护冲突，并在 Unix 创建符号链接、Windows 创建 `.exe` launcher |

---

## 快速开始

### 构建

需要 Rust 1.85 或更高版本；最低工具链同时在 Alpine Linux/musl 环境验证。

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
idlebox printf '%s = %04d\n' answer 42
idlebox env MODE=demo idlebox printenv MODE
idlebox cat -n README.md
idlebox ls --color=auto -lah
idlebox tar -czf source.tar.gz src
idlebox gzip -k report.txt
idlebox unzip archive.zip -d output

# 查看命令帮助与版本信息
idlebox --help
idlebox help wc
idlebox --list
idlebox --version

# 自动安装（为所有 Applet 创建 launcher）
idlebox --install              # Unix: /usr/local/bin；Windows: %LOCALAPPDATA%\IdleBox\bin
idlebox --install ./bin        # 安装到自定义目录
idlebox --install --dry-run ./bin  # 预览但不执行写入
idlebox --install --force ./bin    # 明确替换冲突的文件或链接

# Unix 上通过符号链接调用
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

GitHub Actions 将格式化/lint/文档、Linux/macOS/Windows 原生测试、跨目标可移植性和 Linux Release 体积测量拆分为独立工作流。

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
