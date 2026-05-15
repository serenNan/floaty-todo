# Floaty Todo

> 一个常驻桌面、悬浮置顶的 Markdown 待办清单 —— 直接读写你自己的 `.md` 文件，按「四象限」组织任务，支持多任务源聚合、操作历史与撤销。

Floaty Todo 不是又一个待办 App，它是你现有 Markdown 笔记的一层**轻量视图**。任务始终以纯文本 `- [ ]` 形式存在你自己的 `.md` 文件里，App 只负责解析、聚合、展示和安全回写 —— 你随时可以用任何编辑器直接改文件，App 会即时感知并刷新。

## ✨ 功能特性

- **多任务源聚合** —— 可配置 N 个任务源，每个是一个文件夹（递归扫描所有 `.md`）或单个 `.md` 文件，统一聚合到一个窗口
- **艾森豪威尔四象限** —— 任务按「紧急+重要 / 重要不紧急 / 紧急不重要 / 不紧急不重要」四象限 + 未分类组织，每个象限可独立折叠、独立添加任务
- **任务历史 + 撤销** —— 所有 App 内操作记入 JSONL 事件流，支持 `Ctrl+Z` / `Ctrl+Y` 撤销重做，独立的时间线窗口可浏览历史并双向跳转；外部编辑也会被记录为只读时间线
- **原子安全回写** —— 勾选、编辑、移动任务都通过临时文件原子写入，逐字节保留原行的缩进 / bullet / checkbox 前缀，不破坏你的文件格式
- **实时文件监听** —— 防抖 fs watcher 监听每个源，外部改动即时反映；内置防写回循环机制
- **Hub 镜像目录** —— 可选地把所有任务源镜像到一个中心目录（文件源用硬链接做真双向同步，文件夹源用 NTFS junction / POSIX symlink）
- **快捷动作** —— 一键用 VS Code / 终端 / Claude Code 打开任务源，或在文件管理器中定位
- **toast 反馈** —— 每个操作都有非侵入式气泡反馈，4 种变体，支持悬停暂停
- **源强调色** —— 每个源可设强调色，作为卡片左条 + header 色相
- **多语言 + 主题** —— 内置中 / 英双语，亮色 / 暗色 / 跟随系统三种主题
- **悬浮置顶窗口** —— 默认窗口置顶，方便边工作边看待办

## 🛠 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Vue 3 + TypeScript、Vite 6、Pinia、vue-i18n |
| 后端 | Tauri 2（Rust） |
| 包管理 | npm（前端）/ Cargo（Rust） |

## 🚀 快速开始

### 环境要求

- [Node.js](https://nodejs.org/)（含 npm）
- [Rust 工具链](https://www.rust-lang.org/tools/install)（`rustup`）
- 各平台的 Tauri 系统依赖，见 [Tauri 前置条件](https://tauri.app/start/prerequisites/)

### 开发

```bash
npm install
npm run tauri dev
```

### 生产构建

```bash
npm run tauri build
```

构建产物（安装包 / 可执行文件）位于 `src-tauri/target/release/`。

## 📁 项目结构

```
src/              # Vue 前端（组件、stores、composables、i18n）
src-tauri/        # Rust 后端
  src/parser.rs   # Markdown 任务解析
  src/storage.rs  # 原子文件写入
  src/registry.rs # 内存任务索引
  src/watcher.rs  # 防抖 fs 监听
  src/history.rs  # 操作历史 + 撤销引擎
  src/hub.rs      # Hub 镜像目录
  src/commands.rs # Tauri IPC 命令
```

更详细的模块职责说明见 [`CLAUDE.md`](./CLAUDE.md)，变更记录见 [`CHANGELOG.md`](./CHANGELOG.md)。

## 📝 任务格式

Floaty Todo 读写标准 Markdown 任务行：

```markdown
## 紧急+重要

- [ ] 修复线上登录 bug
- [x] 回复客户邮件

## 重要不紧急

- [ ] 准备季度规划
```

`##` 标题划分四象限分组，`- [ ]` / `- [x]` 是未完成 / 已完成的任务。App 完全兼容你手写的文件，也能把新任务自动追加到对应象限标题下。

## 📄 License

暂未指定。
