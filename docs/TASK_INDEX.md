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
| CHRON-030 / 修复及验证完成 | 单 worker、FIFO/关闭/中断确认、成对发布；[复核收尾](reports/CHRON-030_032_REVIEW_CLOSEOUT.md)、[同工作量对照](reports/CHRON-033_SCALE_BENCHMARKS.md) | ADR0015/0028；变更并发契约或复查证据时重开 |
| CHRON-031 / 实现及修正观测完成 | 真实批量快照与渲染一致性；原 delta FPS 已撤回；最新 mean60.002FPS，有慢帧；[报告](reports/CHRON-031_GODOT_MICRO_WORLD.md)、[真实帧记录](reports/data/chron-031-final-frames.json) | ADR0026/0028；不可声称恒定60FPS；更改呈现或测量方法时重开 |
| CHRON-032 / 修复及十年验证完成 | seed42/100人/3650天，真实移动和吃睡工作100/100；修正后的1次长跑/RSS；[复核报告](reports/CHRON-030_032_REVIEW_CLOSEOUT.md)、[原始结果](reports/data/chron-032-repair-chaos-memory.json) | ADR0027/0028；n=1不代表方差；旧3次timing及不同fixture单次RSS保留为历史 |
| CHRON-033 / 完成 | 五档规模全部2warmups+10timings通过，RSS各一次，100人三种模式同工作量比较；[报告](reports/CHRON-033_SCALE_BENCHMARKS.md) | ADR0029；源/二进制manifest；高档为Core诊断，不代表万人客户端 |
| CHRON-034 / 完成于8dc1595 | 固定seed0/1/42语料；本地和两项托管门禁通过；[报告](reports/CHRON-034_REGRESSION_CI.md)、[CI证据](reports/data/chron-034-hosted.json) | 最终交付须再核对PR最新SHA；不得套用旧green |
| CHRON-035 / 代码及本地验证完成 | 旧共享spike入口退役，真实runner替代，负向API/等价覆盖；[报告](reports/CHRON-035_SPIKE_RETIREMENT.md) | ADR0010 retired；代码源2d050b8；最终双CI见PR2交付记录 |
| CHRON-036 / 报告汇总完成，待产品验收 | [Phase1报告](reports/PHASE_1_MICRO_WORLD_KERNEL_V1.md)、[最终本地门禁](reports/data/chron-035-036-local-validation.json)、[PR2交付记录](https://github.com/GabrielMu2006/Palimpsest/pull/2) | 外部记录给出最终head与CI链接；仍需双检查green；Phase2未授权 |


## 索引维护

- 完成后更新日期/状态/证据/source identity/限制；失败报告保留并标明后继，不删除原始数据。
- 结果索引与新Task不抄旧文档全文；不会因“曾经已批准”跳过当前依赖核验。
- 不把Implementation、Tests green、Performance recorded、Owner accepted混写成同一状态。
- 必读规范仍完整读取；本索引只减少无关历史的重复检索，不是安全/质量豁免。
