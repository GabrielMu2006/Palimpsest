# Task 索引 — 按需读取，不递归加载历史

入口：[CURRENT_PROGRESS](CURRENT_PROGRESS.md)。本索引记录已知状态，不是批准或产品规范。
当前工作只读当前 Task、直接相关 ADR 和实现；历史全文按下表重开条件加载。

| ID / 当前状态 | 简要结果与证据 | 现行约束 / 何时需要全文 |
|---|---|---|
| Phase 0 / owner confirmed 2026-08-29 | [Architecture Spike](reports/ARCHITECTURE_SPIKE_V1.md)：Rust headless、Godot bridge、稳定ID、Scheduler、存储/快照/10K原型 | ADR0001–0010；改变这些边界或复查具体硬件结果时打开 |
| CHRON-018–026 + REM-008A / 历史已交付 | [Remediation](reports/PHASE_1_REVIEW_REMEDIATION_PLAN_V1.md)、[内存证据](reports/REM-008A_MEMORY.md)：grid/terrain/person/needs/sites/path/utility及RSS工具 | ADR0011–0020；改对应模块或复查旧验收时打开相关单项报告，不整批读取 |
| CHRON-027–029 / 实现及V2本地验证完成 | ActionRuntime / WorldKernel / schema2 RenderSnapshot；[V2最终报告](reports/P1_KERNEL_REPAIR_V2.md)含测试/性能证据 | ADR0021–0025；改相关契约或核查证据时打开对应章节，不默认重读三份旧Task |
| P1-KERNEL-REPAIR V1 / 部分实现，验收缺口由V2接管 | [历史计划](tasks/P1_KERNEL_REPAIR_V1.md)、[历史报告及纠正](reports/P1_KERNEL_REPAIR_V1.md)：旧主要复现通过，但八个新增契约探针失败、测量协议缺口 | ADR0024/0025；仅追溯旧决策/错误测量/批准时读全文，不能沿用“全部完成” |
| P1-KERNEL-REPAIR-V2 / locally verified 2026-08-31，已关闭 | [计划](tasks/P1_KERNEL_REPAIR_V2.md)、[报告](reports/P1_KERNEL_REPAIR_V2.md)、[最终源码](reports/data/kfix-v2-environment.json)：debug/release各330执行；40正式+1smoke时间样本、15冷进程；七组缺口闭环 | ADR0025；仅契约修改、回归、证据核查或显式重开时读全文；未完成整个Phase1/hosted CI |
| CHRON-030 / 实现及本地验证完成 2026-08-31 | SimulationWorker 单线程命令桥、有界队列/ack、暂停/六速度/Step/AdvanceTo、独立关闭；[报告](reports/CHRON-030_SIMULATION_WORKER.md)、[计时](reports/data/chron-030-worker-bench.json)、[内存](reports/data/chron-030-worker-memory.jsonl) | ADR0015 Phase1 supplement；改 worker 契约或核查证据时打开报告；Kimi Code 运行时无 Luna 派发，主代理直接实现 |
| CHRON-031 / 实现及本地验证完成 2026-08-31 | Godot 微世界呈现：worker 驱动、批量 snapshot_frame、无损 u64 id、时间控制、指标 overlay；旧 delta FPS 已撤回，修正后 mean59.975 FPS；[报告](reports/CHRON-031_GODOT_MICRO_WORLD.md)、[帧数据](reports/data/chron-031-frames.json) | ADR0026；改呈现/桥接契约或复查 FPS 证据时打开 |
| CHRON-032 / 实现及本地验证完成 2026-08-31 | Headless 10 年混沌：seed42/100人/315,360,000s，3 次确定性运行（min/median/max 1627/1678/1686 s，~188k sim-s/wall-s）；无 panic/NaN/循环/悬垂/无界队列；每人均完成 Eat/Sleep/Work+移动；[报告](reports/CHRON-032_CHAOS_10YEAR.md)、[计时](reports/data/chron-032-chaos.json)、[内存](reports/data/chron-032-memory.jsonl) | ADR0027；改 runner/检测器契约或核查 10 年证据时打开 |
| CHRON-033–036 / 用户已授权，执行中 | [Phase1总计划](PHASE_1_PLAN.md)和各相关Task | 2026-08-31 用户明确要求完成033–036；ADR0028/0029、review closeout为当前入口；Phase2仍未授权 |

## 索引维护

- 完成后更新日期/状态/证据/source identity/限制；失败报告保留并标明后继，不删除原始数据。
- 结果索引与新Task不抄旧文档全文；不会因“曾经已批准”跳过当前依赖核验。
- 不把Implementation、Tests green、Performance recorded、Owner accepted混写成同一状态。
- 必读规范仍完整读取；本索引只减少无关历史的重复检索，不是安全/质量豁免。
