# P1-KERNEL-REPAIR 修复执行 Prompt

本文件是给执行者的指令模板；**文档存在不代表用户已经下令执行**。
用户明确提交以下 prompt 或要求按配套计划实施时，才记录实施批准。
契约与全部边界以 [修复计划](../tasks/P1_KERNEL_REPAIR_V1.md) 为准。

```text
请在 /Users/gabrielmu/Documents/Palimpsest 执行
docs/tasks/P1_KERNEL_REPAIR_V1.md：
Plan ID P1-KERNEL-REPAIR，Revision 2026-08-31-r1，KFIX-001 至 KFIX-008。

我接受该计划的全部推荐决定、API/ADR/调用方/工具配套修改、测试排期和终点。
这包括：拒绝操作无副作用、同刻决策合并、Setup/Running/Faulted 与显式读错误、
当前时刻 Needs 投影、事件轮换前统计/摘要、schema 2 与站点/Needs DTO，
以及完整年度/十样本/RSS 测量、统一验收和当前进度文件。
无需逐项再确认 Task、ADR、普通实施细节或已授权的代理分发。

先完整阅读 MASTER_SPEC.md、AGENTS.md、ARCHITECTURE/PERFORMANCE、该修复计划
和相关 ADR。核对当前真实工作区，保留全部已有 dirty/untracked 修改；旧 HEAD
不是新增代码基线。不要依赖 /tmp 复现文件，按计划重建并注册回归用例。
先记录修复 ADR，再按 DAG 连续实施；每项通过局部验证后继续下一就绪任务。
只做计划内修改，不修改 Master Spec、Utility 权重/需求速率、历史 raw 数据、
native RSS unsafe 边界；不实现 CHRON-030+、不更改存储或 Godot 游戏功能。

审查用一次有针对性的父代理复核，普通任务跑受影响测试，整合后统一执行计划
要求的全量门禁和正式 benchmark；不删除、跳过、弱化测试，不放宽性能预算。
普通失败自行在范围内修复。只有 Master 冲突、实质未计划增量或真正无法解决
的外部阻塞才停下。不得把缺工具/缺测量写成 N/A 或留给我操作后就宣布完成。

默认由 Codex 完成，不使用 OpenCode/opencode-go，不更改我的 provider 配置。
不执行 commit/push/PR/merge、GitHub 设置修改、安装依赖或新聊天创建。

结束前生成 docs/reports/P1_KERNEL_REPAIR_V1.md 和 docs/CURRENT_PROGRESS.md，
记录每个问题的修复与测试证据、正式原始测量、实际修改文件、未完成项和限制。
向我简要说明敏感变化，特别是 Result API、初始化/失败行为、schema 2、事件计数。
全部完成后停止；不把这一轮修复当作 Phase 1 总验收，也不要自动进入下一任务。
```
