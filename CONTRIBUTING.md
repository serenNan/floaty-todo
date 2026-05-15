# 贡献指南 / 开发注意事项

本项目用 **Claude Code** 做 AI 辅助开发。所有协作者请遵循以下规则，保持一致的开发流程。

## 环境准备

| 项 | 说明 |
|---|---|
| OS | Windows 11，主力 shell 为 PowerShell |
| 包管理 | npm（前端）、cargo（Rust） |
| 启动开发 | `npm run tauri dev` |
| 生产构建 | `npm run tauri build` |

技术栈与目录结构见根目录 `CLAUDE.md`。

## Claude Code Skill 配置

本项目约定使用两个 skill，配置方式不同：

### 1. todo skill —— 已内置

`.claude/skills/todo/` 是 skill 实体，随仓库走，clone 下来即可用，无需安装。

- **用途：管理本项目的待办清单。**
- 任何记录、拆分、勾选待办的场景都走它，不要手写零散 TODO。
- 它维护项目根目录的 `./TODO.md`。
- `/todo` 显式触发。
- **约定：** 开发过程中产生的任务统一进 `./TODO.md`，由 todo skill 维护。

### 2. superpowers —— 插件，需自行安装

superpowers 是 Claude Code **插件**（含 14 个开发流程类 skill），不进仓库，各人在自己的 Claude Code 环境安装一次：

```
/plugin install superpowers@claude-plugins-official
```

或用 `/plugin` 菜单挑选安装。上游备用来源：

```
/plugin marketplace add obra/superpowers-marketplace
/plugin install superpowers@superpowers-marketplace
```

安装后重载会话，能看到 `superpowers:using-superpowers`、`superpowers:brainstorming` 等 skill 即成功。

#### superpowers 各 skill 用途

| Skill | 何时用 |
|---|---|
| `using-superpowers` | 入口，约定如何发现与使用 skill |
| `brainstorming` | 加功能 / 改行为前，先做需求与设计探索 |
| `writing-plans` | 有了规格、动代码前，写多步实现计划 |
| `executing-plans` | 在独立会话中执行带审查检查点的计划 |
| `subagent-driven-development` | 当前会话用子 agent 执行含独立任务的计划 |
| `dispatching-parallel-agents` | 2+ 个无共享状态的独立任务并行分派 |
| `test-driven-development` | 实现功能 / 修 bug 前先写测试 |
| `systematic-debugging` | 遇到 bug、测试失败、异常行为时系统排查 |
| `requesting-code-review` | 完成任务 / 合并前发起代码审查 |
| `receiving-code-review` | 收到审查意见、落实修改前做技术校验 |
| `verification-before-completion` | 宣称完成 / 通过前先跑验证命令拿证据 |
| `finishing-a-development-branch` | 实现完成后决定如何集成（merge / PR / 清理）|
| `using-git-worktrees` | 需要与当前工作区隔离时建 worktree |
| `writing-skills` | 创建、编辑、验证 skill |

## 开发流程约定

开发本项目时遵循 superpowers 流程：

1. **先 brainstorm，再动手** —— 任何新功能 / 行为改动，先用 `brainstorming` 厘清需求与设计，不要直接写代码。
2. **多步任务先写计划** —— 用 `writing-plans` 产出计划，再按 `executing-plans` / `subagent-driven-development` 执行，每步带检查点。
3. **TDD** —— 修 bug 先写能复现的测试，加功能先定义"完成"的可验证标准（见 `test-driven-development`）。
4. **bug 走系统排查** —— 遇到报错 / 行为异常用 `systematic-debugging`，不要凭猜测改代码。
5. **完成前先验证** —— 用 `verification-before-completion`，跑 `npm run tauri build` / 相关测试拿到证据再宣称完成。
6. **代码审查** —— 较大改动或合并前走 `requesting-code-review` / `receiving-code-review`。

## 代码风格

- 改代码只清理本次改动产生的未使用 import / 变量，不顺手清 pre-existing dead code。
- 风格与周边代码保持一致，不顺带"改进"无关代码。
- 版本 / 迭代记录进 `CHANGELOG.md`，不写进 `CLAUDE.md`。
