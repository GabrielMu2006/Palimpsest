# P1 Kernel Repair V2 — 验收缺口收尾

- Plan ID: `P1-KERNEL-REPAIR-V2`; revision: `2026-08-31-r1`。
- Status: **Approved / Implemented / Locally verified，2026-08-31**。本次用户明确要求“写相关计划，你自己调用子agent完成修复”，批准本轮七组验收缺口的计划与实施；不再逐项补签。R2-01–06完成，证据见[报告](../reports/P1_KERNEL_REPAIR_V2.md)；不等于整个Phase1或hosted CI验收。
- Context: V1 的 87 项 sim-core + 10 项 Scheduler 测试通过，但独立的八个契约探针失败；V1 不能作为全量验收。旧计划/报告保留并添加纠正入口。
- 终点：本计划六个 Task 验证、最终测量、报告、当前入口及历史摘要索引完成。不进入 CHRON-030+，不 commit/push/PR/修改远端设置、不调用 OpenCode、不修改模型配置。
- 全局禁止：修改 MASTER_SPEC、删除/弱化测试、改权重/速率/地图/寻路算法、存储格式、native RSS unsafe、Godot 产品功能、性能预算或无关代码。

## 已确定决定 / API Contract

详见 [ADR-0025](../adr/ADR-0025-kernel-repair-completion.md)，它补足 ADR-0024，不重写历史。

1. start/cancel 在任何写入之前计算 projected Needs、continuation/check due、token/EventId 余量。每人另记录最近成功动作提交时间，不能仅用 last_needs_at 拒绝倒退。保留懒 Needs 基线，不拷贝世界，不新增 rollback。
2. Kernel metrics 保留返回类型，新增 lifecycle/failed_at 标记，动作队列部分读取最后完整边界的有界缓存；不读取失败 instant 的 live action metrics。next_due 与 sites 改为 Result 并在 Faulted 拒绝；sites 实际含 WorkCounter，不能当作纯静态数据公开。人物投影失败显式 Err，不能 fallback 到旧 Needs。health.last_complete 始终等于 now。
3. KernelAdvance.events 使用本轮上游生成总量，不是缓冲长度。保留累计 digest/drop 恒等式；对照独立 FNV 小向量、容量边界及 drain/分段测试。
4. TerrainBatch / PersonRender 独立 Deserialize 共用自身 validator；完整 DTO 同样调用；Moving 必须是非 Idle 且 kind/target 匹配。维持 schema 2，不增加存档兼容承诺。
5. benchmark parser 拒绝未知/重复/缺值/非法参数、零 samples；两次完整 warmup + 十次正式样本，逐样本原始时间/状态与一致性断言，上中位数 index n/2、min/max，无微秒取整归零。Kernel 记录完整 rounds_total、队列观测最大值和 rates。
6. 明确 Kernel 基准恢复 V1 计划指定的 seed42/default sites/**所有人首个 walkable 出生点**；旧 V1 BFS 分散出生不作可比基线，不宣称优化。Render 保留原 seed42 首批 row-major walkable 出生、实际到600秒，单独说明不同布局。Action 保留 seed25025 分散出生/172800秒。不改 workload 来躲避 bug。
7. Render RSS control/snapshot 共享准备及只读验证；kernel/DTO/bytes 保持到第二次 observe 之后（borrow black_box，不移动释放）。零人口 bytes/person 为 null。RSS 原生 proof 算法不改，保留所有失败样本，不重试筛选。
8. 建立 `CURRENT_PROGRESS -> TASK_INDEX -> 当前Task/相关ADR` 路由。已完成计划默认只读索引摘要；历史全文只在改其契约、排错、核查证据或用户要求时加载。保留完整审计记录及四份必读规范；同一任务上下文已完整读过且文件未变不反复 cat，压缩/新会话遵循必读要求。无 token 节约比例承诺。

## DAG / 所有权

```text
R2-01 动作 (Luna) ─┐
R2-02 Kernel (主) ─┼→ R2-04 工具 (Luna) → R2-05 测量 (主) → R2-06 验收/索引 (主)
R2-03 DTO (Luna) ──┘
```

01/02/03 写文件不重叠。03 先保持现有 Kernel 调用；02 的 sites Result 接线由主代理在03交付停止后机械适配。公共契约由本计划固定，子代理不决策；编译验证由主代理排期，避免共享 target 写竞争。04 在前三项接口稳定/测试通过后分发，测量不与构建/代理重负载并行。

## R2-01 — 拒绝原子性和动作时间水位

- Context / Scope: 收尾上界 start/cancel 和移动后倒退取消；保留 retry、计数、事件、Needs、队列及后续轨迹。
- Out of Scope: Kernel、DTO、AI选择/默认表、Scheduler 重构。
- Dependencies: ADR-0024/0025，现有 ActionConfig/Scheduler.check_schedule_capacity，V1 fixture。
- Files Modified / Allowed: `crates/sim-core/src/actions.rs`，新增 `tests/action_repair_v2.rs`；模块内 cfg(test) 可覆盖 token/event 极值。
- API Contract: start/cancel 签名不变；现有 typed TimeOverflow/Schedule/EventLogExhausted；拒绝零副作用；独立最近提交时刻不替代 Needs 基线。
- Tests: `cargo test --locked -p palimpsest-sim-core --test action_repair_v2`、actions 单元测试、旧 kernel_repair 和 action_closed_loop。至少上界 start/cancel、移动与arrival后倒退、负初始时间、pending retry、重复拒绝、token/event exhaustion；比较前后和未来轨迹。
- Benchmark: R2-05 对最终 actions 测 Action100/1000、Kernel 年度；此 Task 不声称性能通过。
- Definition of Done: 三个动作契约探针通过，扩展负例保持状态/事件一致，所有旧断言保留；Luna 交付后主代理独立复核。

## R2-02 — 完整 Kernel 读边界与计数

- Context / Scope: 故障 metrics/next_due/sites 旁路、投影 fallback、last_complete、单次推进事件总数；加入真实失败 instant 的回归（不是仅强行切状态）。
- Out of Scope: 恢复/rollback、线程/worker、动态人口、Event Store。
- Dependencies: 固定 ADR-0025；ActionRuntime 公共签名不变。
- Files Modified / Allowed: `crates/sim-core/src/{kernel,lib}.rs`，`tests/{kernel,kernel_repair,kernel_repair_v2}.rs`；03停止后对render调用方机械适配；必要只读导出与直接 examples 接线归主代理。
- API Contract: 采用上方决定2/3；先更新完整统计再发布边界，Faulted只能读带故障标签的已提交诊断/旧事件，不泄露WorkCounter半提交。
- Tests: `cargo test --locked -p palimpsest-sim-core --test kernel_repair_v2 --test kernel`；合法极大duration在成功round之后失败；动态读Err/metrics缓存/health标记；4095/4096/4097事件与容量1、分段/drain一致；独立FNV oracle。
- Benchmark: R2-05。
- Definition of Done: 已知两个探针（fault metrics / advance events）通过；任何公开动态读无半提交旁路；现有调用方编译。

## R2-03 — DTO 自身校验闭包

- Context / Scope: 独立 TerrainBatch/PersonRender decode 与 Moving Idle 的三个探针。
- Out of Scope: schema3、Godot、数据导入执行、改变Kernel/API。
- Dependencies: ADR-0024/0025 D4，既有 DTO；Kernel 接线最终由主代理处理。
- Files Modified / Allowed: `crates/sim-core/src/render.rs`，新 `tests/render_repair_v2.rs`。
- API Contract: schema2，使用私有 wire 和 shared validator；独立 batch维度/cells、person action-state-target、existing EntityId/Needs/coord 验证不绕过。
- Tests: `cargo test --locked -p palimpsest-sim-core --test render_repair_v2 --test render`；维度0/127/129/usizeMAX、短长cells、所有动作合法组合/非法配对、完整快照和独立decode、合法空世界。
- Benchmark: R2-05完整 schema2 build/serialize/RSS。
- Definition of Done: 三个DTO探针通过，builder/root/独立decode复用不变量，无旧测试删减。

## R2-04 — 可验证的测量工具

- Context / Scope: 补齐 raw/CLI/median/rounds/rates/queue、同样本一致性、RSS存活与control、短adapter测试；不执行年度测试于普通unit gate。
- Out of Scope: native RSS算法/unsafe、游戏语义、依赖版本、旧数据覆盖。
- Dependencies: 01/02/03已独立验证并冻结最终公共接口；ADR-0020/0025。
- Files Modified / Allowed: `crates/sim-core/examples/{action_execution_bench,kernel_bench,render_snapshot_bench}.rs`，新增 `examples/support/*` private helpers；`tests/benchmark_protocol_v2.rs`；`tools/bench-memory/{src/main.rs,tests/cli.rs,README.md}`。manifest只有主代理审批后必要现有依赖接线；不得改Cargo.lock外部版本。
- API Contract: 决定5/6/7；兼容原合法flags，支持--json；fixture/config/units/median规则清晰；固定结果校验不运行时学习golden；kernel max_queue是完整边界观测值，不冒称逐item峰值；snapshot各字段回读一致。
- Tests: examples / benchmark_protocol_v2；release memory cli原测试加三新case协议短测试。非法samples/缺值/未知/重复、upper median偶数/亚微秒、零人口null、两observe存活结构、原goldens不变。
- Benchmark: 测量由05统一执行；新年度case不得进入unit test。
- Definition of Done: 三种工具输出可机器校验原始样本、min/median/max及完整指标；无静默fallback/假checksum/no-op测量；工具文档同步。

## R2-05 — 最终源码性能证据

- Context / Scope: 一次串行采集M5 16GB、release、最终源码；保存 hash/硬件/命令和逐样本数据。
- Out of Scope: 10年gate、FPS/Core+Client预算声明、其他规模/重测未变化旧模块。
- Dependencies: 01–04验证，最终源码冻结；旧V1 raw只读。旧fixture不可比项明确首次正式基线。
- Files Modified / Allowed: 新 `docs/reports/data/kfix-v2-*`，必要 `tools/collect-kfix-v2.*`、修复报告。
- API Contract: 2warmups+10timing；5case各3独立冷进程；cold不证明则不完成，prepared不证明null；无重试筛选。
- Tests: 数据schema/样本数/一致性/median/rate/尺寸/RSS proof/源码hash核对，保留失败输出。
- Benchmark: Action100/1000 172800秒；Kernel100 86400秒smoke然后31536000秒；Render100到600秒；memory action100/1000原86400秒、kernel100year、rendercontrol100/snapshot100。所有目标实际Reached、population/identity/Needs/events一致。命令为旧V1的同名binary/flags，raw前缀改kfix-v2；严禁在timing时并发编译。
- Definition of Done: 三份timing raw和memory raw及环境/source identity完整，可复算；报告baseline可比性和限制，不把运行中标完成。

## R2-06 — 一次验收与上下文路由

- Context / Scope: 纠正V1全完成断言/round单位，建立当前入口/已完成摘要，最终交付。
- Out of Scope: 删除历史、改变Master必读、全局skill/provider设置、自动开始030。
- Dependencies: 01–05；可提前写路由草案，只有证据完成才能标Verified。
- Files Modified / Allowed: `AGENTS.md`、`docs/{CURRENT_PROGRESS,TASK_INDEX,EXECUTION_CONTRACT,ARCHITECTURE,PERFORMANCE,PHASE_1_PLAN,PHASE_1_REMAINING_EXECUTION}.md`；本计划；V1计划/报告仅追加纠正标记；`docs/tasks/CHRON-027.md`–`029.md`和其reports局部证据索引；ADR0021–24仅追加后继链接；新报告 `docs/reports/P1_KERNEL_REPAIR_V2.md`。
- API Contract: N/A文档。索引记录ID/状态/结果/现行ADR/证据/重开条件；当前索引不复制历史全文。计划按证据关闭，历史批准不自动扩大。
- Tests: `./tools/ci-rust.sh`；`cargo test --release --locked --workspace --all-targets --all-features`；`cargo test --locked --workspace --doc`；`RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`；metadata/tree exact normal graph；`./tools/ci-godot.sh`现有smoke；diff check、文档链接/必需字段、baseline文件清单/Master hash。
- Benchmark: 复用05最终源码，不重复纯文档测量。
- Definition of Done: 七组问题逐条证据闭环，子代理diff及关键回归由主代理独立验证；97旧测试不削弱；保留本地dirty事实、hostedCI未验证、Phase1十年仍未完成；报告敏感变更及上下文策略限制。

## 分发与暂停

Luna使用内置子代理、请求gpt-5.6-luna medium、无全聊天fork、不得递归。动作/DTO/工具目标已诊断且可测试；主代理持有Kernel故障/跨模块契约/证据/最终验收。最多一次具体返工，再由主代理接管。常规测试失败继续范围内修复；仅Master冲突、实质未计划扩展或真实阻塞才停。
