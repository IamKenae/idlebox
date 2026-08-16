# Contributing to IdleBox / 贡献指南

First off, thank you for considering contributing to IdleBox! It's people like you that make IdleBox such a great tool.

首先，感谢您考虑为 IdleBox 贡献代码！正是因为有您的参与，IdleBox 才能变得更好。

---

## Branching Strategy (GitFlow) / 分支策略

We strictly follow a modern GitFlow branching model to keep our history clean and our releases stable. 
我们严格遵循现代 GitFlow 分支模型，以保持历史记录的整洁和发布版本的稳定。

### The Main Branches / 主干分支
- **`main`**: The primary branch where the source code of `HEAD` always reflects a production-ready state. Do not push directly to this branch. 
  代表生产就绪状态的绝对主干分支。禁止直接推送到此分支。
- **`develop`**: The primary integration branch. All feature branches should branch off from `develop` and be merged back into `develop` via Pull Requests.
  主要的集成开发分支。所有的特性分支都应从 `develop` 检出，并通过 PR 合并回 `develop`。

### Supporting Branches / 辅助分支
When starting new work, branch off from `develop` and prefix your branch name with its type:
当您开始新工作时，请从 `develop` 检出新分支，并使用以下前缀命名：

- **`feat/`** (e.g., `feat/network-applets`): For developing new features or applets. / 开发新功能或新工具。
- **`fix/`** (e.g., `fix/md5sum-stdin`): For fixing bugs. / 修复 Bug。
- **`docs/`** (e.g., `docs/update-readme`): For documentation-only changes. / 仅修改文档。
- **`refactor/`**: For code changes that neither fix a bug nor add a feature. / 代码重构（不改变外部行为）。
- **`chore/`**: For updates to the build process, auxiliary tools, or libraries. / 构建过程、辅助工具或依赖项的更新。

---

## Pull Requests / 提交代码

1. **Base your PR on `develop`**: All pull requests must target the `develop` branch.
   **目标分支**：所有的 Pull Request 都必须提交到 `develop` 分支。
2. **Pass all checks**: Ensure `cargo fmt`, `cargo clippy`, and `cargo test` pass successfully.
   **通过检测**：确保代码能完美通过格式化、静态检查和所有单元测试。
3. **Wait for Review**: Code review is mandatory. Pull Requests cannot be merged until they receive at least one approval.
   **代码审查**：代码审查是强制性的，PR 必须获得至少一个 Approve 才能合并。

---

## Commit Message Guidelines / 提交信息规范

We follow Conventional Commits format for commit messages:
我们遵循 Conventional Commits (约定式提交) 规范：
```
<type>[optional scope]: <description>
```
Examples / 示例：
- `feat(hash): implement md5sum applet`
- `fix(b3sum): handle parallel processing edge cases`
- `docs: update installation instructions`

Thank you for contributing!
感谢您的贡献！
