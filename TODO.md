# TODO

> Floaty Todo 自家的待办。文件本身就是一个合法的 file source —— 把这个项目目录 / 本文件加进
> Floaty Todo 就能直接在悬浮窗里管理它，等于 dogfooding。

## 🔴 紧急+重要

- [x] **dev-test 这一轮新功能**：collapse-all、drag-to-reorder、reveal-in-explorer、hub-reveal、全 header click、宽度翻倍 —— 跑 `npm run tauri dev` 实际过一遍，确认没有视觉残留
- [x] 修复拖动bug

## 🟡 重要不紧急

- [ ] **[v0.3] 全局快捷键**：呼出主窗口 + 快速添加任务（`tauri-plugin-global-shortcut`），让 Floaty 真正"按一下就能记一条"
- [ ] **[v0.3] sidecar 元数据**：`.floaty-todo.json` 记录 `completed_at`；UI 实现「当天完成划线灰显 / 隔天默认隐藏」（PLAN.md 第七节流程 E）
- [x] **[v0.3] 搜索 / 过滤**：顶部输入框旁加搜索图标，支持按文本 fuzzy 匹配，多 source 跨范围
- [ ] **[v0.3] 点击任务跳 Obsidian**：`obsidian://` URL scheme 打开任务所在文件 + 行
- [ ] **[v0.3] 子任务缩进 + 父任务勾选传播**：勾父任务时弹 toast 询问「是否一并勾选 N 个子任务」（PLAN.md Q2）
- [ ] **[v0.3] Obsidian Tasks emoji 只读识别**：⏫ / 📅 / 🔁 等元数据解析为 UI 徽章，不改原文
- [ ] **[v1.0] 拖拽排序整个 source**（不只是 quick-action 按钮）—— 让用户调整 source 在 TaskList 里的显示顺序
- [ ] **[v2.0] 配套 Claude Code skill (todo skill)**：写一个标准 skill 教 Claude 怎么遵守 PLAN.md 第十节的约定（行号稳定 / 不加 emoji 元数据等）

## 🟠 紧急不重要

- [ ] **Hub label 冲突**：两个 source label 一样时，第二个 mirror 会失败（hard link 目标已存在）。当前回 `CommandFailed`；理想行为：后端自动加 `(2)` 后缀，或弹 confirm 让用户重命名
- [ ] **跨卷 source 加进 hub 报错没在 UI 显示**：`add_source` 用 `try_hub` 吞了错误，用户不知道为什么 hub 里没出现。考虑加一个 toast / inline 提示
- [x] **大文件夹首次扫描进度**：现在只有"扫描中..."文字 + 旋转图标，没有 N/M 计数。`spawn_source_scan_and_watcher` 里 emit progress events
- [ ] **TitleBar.vue 孤儿组件**：commit d74fba0 引入后 1c3a1e2 弃用了，文件还在。该删

## 🟢 不紧急不重要

- [ ] **[v1.0] 持久化折叠状态**：现在重启全部回到展开。考虑 localStorage `floaty.collapsed.<source_id>`
- [ ] **[v1.0] 任务排序选项**：按完成状态 / 按 source / 按文件 / 按行号；目前固定 sortedTasks 是 undone-first
- [ ] **[v1.0]「已归档」面板**：单独 view 展示历史已完成（依赖 sidecar 的 completed_at）
- [ ] **[v1.0] Obsidian Tasks 完整支持**：按截止日期排序、优先级颜色、循环任务自动复刻
- [ ] **[v2.0] 可选 MCP server 模式**：把 hub-folder 数据通过 MCP 暴露给 AI 客户端
- [ ] **[v2.0] 任务自动分类建议**：未做完任务过多时建议拆分 / 归类
- [ ] **inbox.md 文件名 hardcoded**：所有 folder source 共用同一个 `inbox.md` 文件名。考虑每个 source 可单独配置
- [ ] **i18n 字符串 hardcoded 在 useConfirm**：confirm.removeSource* 等键值散在 useConfirm 调用方，提一个统一 helper
- [ ] **dev 端口 1422 写死**：之前为避开 WishTalk 的 1420 改的；可以做一个端口自适应
- [ ] 添加一个自动总结功能

## 设计上明确「不做」

- ❌ 云同步 —— 用户自己 Obsidian Sync / Syncthing / OneDrive
- ❌ 完整 markdown 渲染 —— 只渲染任务行的 inline md（粗体 / 斜体 / 代码 / 链接 / 删除线）
- ❌ 看板 / 甘特图 / 项目管理 —— Floaty 是悬浮 todo，不是 Notion
- ❌ 跨文件移动任务 —— 会破坏 line_number 稳定性，sidecar 会失联
- ❌ AI 内嵌（应用本身不调 LLM） —— AI 通过文件 / hub-folder 介入
