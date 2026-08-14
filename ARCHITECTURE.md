# IdleBox 架构设计方案

## 1. 项目目录结构 (Directory Structure)

```
idlebox/
├── Cargo.toml                 # 项目配置与极致体积优化
├── Cargo.lock                 # 依赖锁定
├── LICENSE                    # Apache-2.0 许可证
├── README.md                  # 项目说明
├── ARCHITECTURE.md            # 架构设计文档
├── src/
│   ├── main.rs                # 主入口：路由分发
│   ├── core/
│   │   ├── mod.rs             # Core 模块导出
│   │   ├── applet.rs          # Applet Trait 定义
│   │   └── dispatcher.rs      # Applet 分发器
│   └── applets/
│       ├── mod.rs             # Applet 模块注册
│       ├── echo.rs            # POSIX echo 实现
│       └── relax.rs           # IdleBox 专属摸鱼指令
└── tests/
    └── integration_test.rs    # 集成测试
```

**设计原则**：
- **Core 与 Applet 解耦**：Core 负责路由分发，Applet 独立实现业务逻辑
- **零成本抽象**：使用静态注册表，避免运行时反射开销
- **单二进制**：所有功能编译进单一可执行文件，通过 symlink 调用

---

## 2. 极致体积压榨的 Cargo.toml

```toml
[package]
name = "idlebox"
version = "0.1.0"
edition = "2021"
authors = ["IdleBox Contributors"]
license = "Apache-2.0"
description = "A modern, lightweight BusyBox alternative written in Rust"
repository = "https://github.com/IamKenae/idlebox"
keywords = ["busybox", "toolbox", "embedded", "minimal"]
categories = ["command-line-utilities", "os"]

[dependencies]
# 无外部依赖，纯标准库实现

[profile.release]
# 极致体积优化
opt-level = "z"           # 优化体积（-Oz）
lto = true                # 链接时优化，跨 crate 内联
codegen-units = 1         # 单线程编译，最大化优化
panic = "abort"           # panic 时直接终止，移除 unwind 代码
strip = true              # 剥离符号表和调试信息
debug = false             # 不生成调试信息

[profile.release.build-override]
opt-level = "z"
codegen-units = 1
```

**预期体积**：
- Release 模式下约 200-400 KB（取决于启用的 applet 数量）
- 使用 `upx` 压缩后可进一步缩小至 100-200 KB

---

## 3. Core 核心设计方案

### 3.1 Applet Trait 定义

```rust
/// Applet Trait：所有子命令的统一接口
pub trait Applet {
    /// 返回 applet 名称（用于路由匹配）
    fn name(&self) -> &'static str;
    
    /// 返回简短描述（用于帮助信息）
    fn description(&self) -> &'static str;
    
    /// 执行 applet
    /// 
    /// # Arguments
    /// * `args` - 命令行参数（不包含 argv[0]）
    /// 
    /// # Returns
    /// * `Ok(i32)` - 退出码
    /// * `Err(Box<dyn Error>)` - 错误信息
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>>;
    
    /// 打印帮助信息
    fn help(&self) {
        println!("Usage: {} [OPTIONS]", self.name());
        println!("{}", self.description());
    }
}
```

**设计要点**：
- 返回 `i32` 退出码，符合 POSIX 规范
- 使用 `Box<dyn Error>` 简化错误处理
- 默认 `help()` 实现，applet 可覆盖

### 3.2 Applet Dispatcher

```rust
/// 静态 Applet 注册表
pub struct Dispatcher {
    applets: Vec<Box<dyn Applet>>,
}

impl Dispatcher {
    /// 创建分发器并注册所有 applet
    pub fn new() -> Self {
        let mut dispatcher = Self {
            applets: Vec::new(),
        };
        dispatcher.register_all();
        dispatcher
    }
    
    /// 注册所有 applet
    fn register_all(&mut self) {
        // 静态注册，编译时确定
        self.register(Box::new(EchoApplet));
        self.register(Box::new(RelaxApplet));
    }
    
    /// 注册单个 applet
    fn register(&mut self, applet: Box<dyn Applet>) {
        self.applets.push(applet);
    }
    
    /// 根据名称查找并执行 applet
    pub fn dispatch(&self, name: &str, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        for applet in &self.applets {
            if applet.name() == name {
                return applet.run(args);
            }
        }
        Err(format!("idlebox: applet not found: {}", name).into())
    }
    
    /// 列出所有可用 applet
    pub fn list_applets(&self) {
        for applet in &self.applets {
            println!("{:<12} {}", applet.name(), applet.description());
        }
    }
}
```

**分发机制**：
1. 从 `std::env::args()` 获取 `argv[0]`
2. 提取文件名（处理绝对路径和 symlink）
3. 在注册表中线性查找（applet 数量少，O(n) 足够快）
4. 支持两种调用方式：
   - `idlebox echo hello` → argv[0] = "idlebox", args = ["echo", "hello"]
   - `./echo hello` → argv[0] = "echo", args = ["hello"]（通过 symlink）

---

## 4. Applet 模块实现范例

### 4.1 src/applets/echo.rs

```rust
use crate::core::Applet;

pub struct EchoApplet;

impl Applet for EchoApplet {
    fn name(&self) -> &'static str {
        "echo"
    }
    
    fn description(&self) -> &'static str {
        "Print text to standard output"
    }
    
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        let mut newline = true;
        let mut start_idx = 0;
        
        // 处理 -n 参数（不换行）
        if !args.is_empty() && args[0] == "-n" {
            newline = false;
            start_idx = 1;
        }
        
        // 拼接所有参数
        let output = args[start_idx..].join(" ");
        
        if newline {
            println!("{}", output);
        } else {
            print!("{}", output);
        }
        
        Ok(0)
    }
}
```

**POSIX 兼容性**：
- 支持 `-n` 参数（不换行）
- 默认在末尾添加换行符
- 参数间用空格分隔

### 4.2 src/applets/relax.rs

```rust
use crate::core::Applet;
use std::thread;
use std::time::Duration;

pub struct RelaxApplet;

impl Applet for RelaxApplet {
    fn name(&self) -> &'static str {
        "relax"
    }
    
    fn description(&self) -> &'static str {
        "IdleBox special: take a break and relax"
    }
    
    fn run(&self, args: &[String]) -> Result<i32, Box<dyn std::error::Error>> {
        // 默认休眠 5 秒
        let seconds = if !args.is_empty() {
            args[0].parse::<u64>().unwrap_or(5)
        } else {
            5
        };
        
        println!("☕ Relaxing for {} seconds...", seconds);
        println!("   (Press Ctrl+C to abort)");
        
        thread::sleep(Duration::from_secs(seconds));
        
        println!("✓ Refreshed! Back to work.");
        Ok(0)
    }
    
    fn help(&self) {
        println!("Usage: relax [SECONDS]");
        println!();
        println!("{}", self.description());
        println!();
        println!("Arguments:");
        println!("  SECONDS    Duration to relax (default: 5)");
        println!();
        println!("Examples:");
        println!("  relax        # Relax for 5 seconds");
        println!("  relax 10     # Relax for 10 seconds");
    }
}
```

**特色功能**：
- IdleBox 专属 applet，体现项目个性
- 支持自定义休眠时长
- 友好的用户提示

---

## 5. 主入口 src/main.rs

```rust
mod core;
mod applets;

use std::env;
use std::path::Path;
use std::process;

use crate::core::Dispatcher;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // 提取 argv[0] 的文件名部分（处理绝对路径和 symlink）
    let argv0 = Path::new(&args[0])
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("idlebox");
    
    let dispatcher = Dispatcher::new();
    
    // 判断调用方式
    let (applet_name, applet_args) = if argv0 == "idlebox" {
        // 方式 1: idlebox <applet> [args...]
        if args.len() < 2 {
            print_usage(&dispatcher);
            process::exit(0);
        }
        (args[1].as_str(), &args[2..])
    } else {
        // 方式 2: ./<applet> [args...] (通过 symlink)
        (argv0, &args[1..])
    };
    
    // 特殊命令：list
    if applet_name == "list" {
        println!("Available applets:");
        dispatcher.list_applets();
        process::exit(0);
    }
    
    // 分发执行
    match dispatcher.dispatch(applet_name, applet_args) {
        Ok(exit_code) => process::exit(exit_code),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

fn print_usage(dispatcher: &Dispatcher) {
    println!("IdleBox v0.1.0 - A modern BusyBox alternative");
    println!();
    println!("Usage:");
    println!("  idlebox <applet> [args...]    # Run an applet");
    println!("  idlebox list                  # List all applets");
    println!("  ./<applet> [args...]          # Run via symlink");
    println!();
    println!("Available applets:");
    dispatcher.list_applets();
}
```

**关键特性**：
- 支持两种调用方式（直接调用和 symlink）
- 未知 applet 输出标准错误信息
- `idlebox list` 列出所有可用 applet
- 符合 POSIX 退出码规范

---

## 附录：扩展指南

### 添加新 Applet

1. 在 `src/applets/` 创建新文件，如 `ls.rs`
2. 实现 `Applet` trait
3. 在 `src/applets/mod.rs` 中导出
4. 在 `Dispatcher::register_all()` 中注册

```rust
// src/applets/ls.rs
pub struct LsApplet;
impl Applet for LsApplet { /* ... */ }

// src/applets/mod.rs
pub mod ls;

// src/core/dispatcher.rs
fn register_all(&mut self) {
    self.register(Box::new(EchoApplet));
    self.register(Box::new(RelaxApplet));
    self.register(Box::new(LsApplet));  // 新增
}
```

### 编译优化

```bash
# Release 构建
cargo build --release

# 查看二进制体积
ls -lh target/release/idlebox

# 进一步压缩（可选）
upx --best target/release/idlebox

# 创建 symlink
cd target/release
ln -s idlebox echo
ln -s idlebox relax

# 测试
./echo "Hello, IdleBox!"
./relax 3
```

---

**设计总结**：
- ✅ 零外部依赖，纯标准库
- ✅ 极致体积优化（~300KB）
- ✅ 模块化设计，易于扩展
- ✅ POSIX 兼容，支持 symlink 调用
- ✅ Apache-2.0 许可证
- ✅ 完整的错误处理和帮助系统
