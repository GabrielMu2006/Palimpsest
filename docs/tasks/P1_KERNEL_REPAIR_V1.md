# CHRON-027–029 修复任务计划 V1

> 历史计划正文保留。后续实施未完整通过独立验收，2026-08-31起由
> [P1-KERNEL-REPAIR-V2](P1_KERNEL_REPAIR_V2.md) 接管未闭环部分。
> 当前任务按V2读取；不要把下方最初Proposed或旧报告Implemented当作当前完成状态。

- **Plan ID**：`P1-KERNEL-REPAIR`；**Revision**：`2026-08-31-r1`。
- **Task 集合**：`KFIX-001` 至 `KFIX-008`，只修复本次审查的问题及其验收缺口。
- **状态**：Proposed。当前请求只授权制定任务文档，不代表代码已经修复或验收通过。
- **批准语义**：用户对本计划说“按照你的计划来”或提交配套
  [修复 prompt](../prompts/P1_KERNEL_REPAIR_V1.md)，即一次接受下文推荐决定、
  必要 API/ADR/调用方/工具修改、测试排期和交付终点；不逐项补签。
- **外部操作**：仅本地文件与本地验证；不 commit/push/PR/merge、不修改 GitHub 设置、
  不安装依赖或改变模型/provider 配置、不创建新聊天。仓库继续公开，main 保护不变。
- **终点**：8 项修复和验证完成，生成修复报告及 `docs/CURRENT_PROGRESS.md`，
  返回结果。即使其他历史计划已批准，也不借本次修复自动开始 CHRON-030–036 或 Phase 2。

遵循只读 `MASTER_SPEC.md`、[AGENTS](../../AGENTS.md)、
[执行契约](../EXECUTION_CONTRACT.md)、[任务模板](TEMPLATE.md)。
这是对 027–029 的定向修复，不是重新实施 Phase 1。

## 1. Context / 已核对起点

2026-08-31 审查的工作区：分支 `phase-1-planning`，HEAD
`e5b0aeb676372a123dd8c27190e94b6a606d498c`，包含大量**未提交且部分未跟踪**的
018–029/REM-008A 实现和文档。HEAD 不是当前实现快照；禁止 reset、stash 覆盖、
只检出旧 HEAD 后声称测了当前代码。实施前重新记录状态并保留已有修改。

审查时的源码身份（执行时变化则检查差异，不覆盖新工作）：

| 文件 | SHA-256 |
|---|---|
| `crates/sim-core/src/actions.rs` | `ac9cd03f0f607297c2ec2259c46a88b62f36bf27279aa579e36b7c97e4a2305a` |
| `crates/sim-core/src/kernel.rs` | `8e42233a9b80c4e29a706e6fd2ab48dfb66cc930555830b304dc0edd5f317aa5` |
| `crates/sim-core/src/render.rs` | `9c837181c057092b2a38785fc541beb01c460fc5c82ba98e6ff0ef47578005bb` |
| `MASTER_SPEC.md`，只读 | `a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea` |

当时实际通过：sim-core 的 61 项 unit/integration/example 测试、2 项 doctest、
fmt、该 crate 的 Clippy、`git diff --check`。不是全 workspace/release/Godot 新验收。
仓库外 7 个契约复现均失败，说明已有测试覆盖不足；不能因此删改旧断言。
临时脚本曾位于 `/tmp/palimpsest-review-EzvXu4/src/lib.rs`，**不将其存在作为执行依赖**；
下表和 §2 已给出可重建输入与期望。

### 问题到责任 Task 的完整映射

| ID | 已确认行为 / 缺口 | 修复责任 |
|---|---|---|
| F01 / P1 | Work 完成与 CriticalCheck 同刻，第二次 start 报 AlreadyExecuting；人物已变而时钟未提交 | KFIX-002、003 |
| F02a / P2 | 非零时刻的 Blocked start 先修改 Needs，再返回失败 | KFIX-001 |
| F02b / P2 | 倒退时间 cancel 返回错误，但先删除了动作/token | KFIX-001 |
| F03 / P2 | 第 100 秒的 KernelPersonView 仍返回 Needs 0/0，而非 100/200 | KFIX-004 |
| F04 / P2 | 4,097 个同刻事件只计 4,096，丢弃计数仍为 0 | KFIX-005 |
| F05 / P2 | 时钟 100 时 start_world(0) 成功，产生 due=1 的过去工作 | KFIX-003 |
| F06 / P2 | 0×0 尺寸配 16,384 cells 可以解码并通过 validate | KFIX-006 |
| G01 / 交付缺口 | DTO 缺少 Task 明列的 ActivitySite/Needs；完整边界读取无失败接口 | KFIX-003、006 |
| G02 / 交付缺口 | 028/029 只有单样本 pilot；年度/十样本/RSS adapter 与正式证据未完成 | KFIX-007 |
| G03 / 状态缺口 | 计划/Task/报告的 Proposed、Implemented、无阻断等表述与实际验收状态不一致 | KFIX-008 |

F01 用合法的**非默认** Work 时长构造，未声称默认配置必在同一秒报错。
F04 是诊断缓冲容量边界测试，不是宣布 Phase 1 已支持 4K 或通过正式扩容验收。

## 2. 可重建的回归 fixture

不要修改现有 ADR-0018 fixture 或 golden；为本次修复增加独立小 fixture：

1. `WorldMap::generate(WorldSeed::new(25_025), WorldGenConfig::default())`。
2. 按现有 `map.local().coords()` 的 row-major 顺序，找第一个完整可行走的 3×3 区域左上角 O。
   使用 `LocalCoord::new` 和 `TerrainKind::is_walkable` 验证，不能手工绕过坐标/地形约束。
3. Work 在 O，Meal 在 O+(2,0)，Rest 在 O+(0,2)；用 `ActivitySite::new` /
   `ActivitySites::new` 创建。Person 出生于 O、Needs 默认 0/0；多人允许重合。
4. 默认 Weights、零 perturbation、默认 PathConfig。通过真实 selector 启动 Kernel。
   Work 零距离到达仍耗 1 秒，默认第一次 Work 完成在第 1,801 秒。
5. F01 只把 Work duration 设为 44,999 秒：1 秒到达 + 44,999 秒工作 = 45,000；
   同时 fatigue 以 raw 2/s 达到临界 90,000。期望恰好一次 Work 完成、一次后续
   Sleep 启动，无 AlreadyExecuting、无伪造 interrupted Work，时钟提交到 45,000。
6. F02a 用尚无动作的 Person，在第 10 秒请求 Eat(O)，O 是 Work 而不是 Meal。
   期望 Blocked，所有原有状态不变。F02b 用 `ActionRuntime` 在第 10 秒启动 Idle，
   再 `cancel(..., External, 0)`；期望时间错误，Idle 和原 token 保持。
7. F03 默认 Kernel 推进到 100：Person 的投影需求为 raw 100/200；读取不写回 ECS。
8. F04 使用 4,097 人，推进到 1,801：有效完成事件总计 4,097；未主动 drain 时，
   默认缓冲保留 4,096、丢弃 1；不是修改缓冲常量来容纳这个测试。
9. F06 从合法快照 JSON 分别修改 width/height/cells；0×0+16,384 cells 必须拒绝。

新测试注册在 Cargo 会发现的 `crates/sim-core/tests/kernel_repair.rs`，公共 fixture
允许放 `crates/sim-core/tests/common/repair_fixture.rs` 并显式 mod 引入。
私有状态、token exhaustion、故障注入断言放相应源模块的 `#[cfg(test)]` 内，
不能为测试开放 ECS handle 或生产可调用的故障注入 API。

## 3. 随计划接受的修复决定

这些是**建议，尚未实施**。执行本计划时主代理先写一份修复 ADR，建议文件
`docs/adr/ADR-0024-phase-1-kernel-repair-contract.md`；若编号已被占用，使用下一空号。
其明确增补/局部取代 ADR-0021/0022/0023，不重写它们的历史决定。
ADR 落字和以下已明确语义不再请求单独批准。

### D1 — 拒绝无副作用，致命执行错误显式停止

- `start/cancel` 对非法身份、目标、时间、重叠和可预检的算术/编号耗尽，先校验，
  再提交。错误不能先取消 retry、写 Needs、删除动作、消费 EventId 或改变队列。
- 用私有 prepared transition/checked values 将一次操作的 fallible 工作放在提交之前；
  仅暂存当前人物/动作所需数据。**不克隆整个 ECS/world，不建立通用事务/回滚框架**。
- 若多个 follow-up token 需要共同预检，允许唯一且狭窄的 Scheduler 公共扩展：
  `check_schedule_capacity(count: usize) -> Result<(), SchedulerError>`，只检查 token/order
  余量、不变更队列。单线程调用期间没有插入者，预检与提交必须覆盖同一确切申请数。
  记录在本修复 ADR；不更换 Scheduler、FIFO、token 格式或增加并发 reservation 系统。
- 内部 due-work 的致命失败遵循 D3：不把未完成边界伪装为成功，也不要求回滚已完成历史。
  预期的 blocked/failed 活动恢复仍沿用 Idle + 正延迟 retry，不升级为全世界致命错误。

### D2 — 同人同刻只决策一次

- 原始 due work 及 outcome events 仍按 due-time/FIFO 执行，不调整原 Scheduler 顺序。
- 一个完整 due instant 执行完后，按 `(EntityId, SimInstant)` 合并**决策请求**；
  多人之间保留请求第一次出现的顺序，不因 HashMap/按 ID 重排而改变决策顺序。
- 有 Completed/Retry 时执行一次重新选择；仅 CriticalBoundary 时才比较并按需中断。
  同刻已完成的动作不得再被算作 interrupted；选择读取该 instant 的最终 Needs，
  包括已完成 Eat/Sleep 的 relief。
- 合并逻辑由 ActionRuntime/共同 driver 的一处实现提供给 Kernel 和 `run_until`；
  不能只在 Kernel catch/忽略 AlreadyExecuting，也不能只改测试 fixture 来避开同刻。
  不合并不同 instant，不合并真实 outcome events，不改变五种动作、权重或时长默认值。

### D3 — 最小生命周期与失败可见性

- Kernel 具有 `Setup / Running / Faulted` 三种状态，新建时 Setup、时钟 epoch。
  `spawn_person` 只在 Setup 的 epoch 允许；本次不实现运行中出生/增员。
- 保留 `start_world(at)` 入口，但仅允许 Setup 且 `at == now() == EPOCH`，成功后 Running；
  再次调用、提前/倒退时间均返回明确错误且不变更人物、allocator、queue、clock。
- Setup 非空世界在未 start 时，不允许正向 advance，返回 NotStarted；equal-target
  可作无副作用 no-op。Setup 空世界允许第一次正向 advance 进入 Running 并直接到目标，
  保留现有 empty-world 测试。随后 start/spawn 被拒绝，而非往过去补工作。
  这是明确的初始化约束收紧；F05 的旧调用序列现在应在更早的非法 advance 处拒绝。
- 可恢复的入参拒绝不置 Faulted。真正的执行/决策/编号耗尽错误记录
  `last_complete`、`failed_at` 和 typed cause，置 Faulted；此后禁止继续变更该 Kernel。
  Phase 1 通过重新创建世界重新运行，不提供“清掉错误继续”的接口，不自动吞错或重试。
- `now()` 永远表示最后完整边界；计数、最新 trace 和向外 drain 的事件不能把失败
  instant 的部分结果计为已完成。此前完整边界的工作与计数保留，不回退整段历史。
- 所有能读取**动态世界当前状态**的入口都检查完整边界；Faulted 不得读到半提交人物/trace。
  静态 map/固定人数、带故障标识的 health，以及此前完整边界事件仍可诊断。
  不允许只保护 RenderSnapshot，却保留另一个无校验的公开 live-state 旁路。
- 推荐公开读 API：`person(id) -> Result<Option<KernelPersonView>, KernelReadError>`、
  `persons() -> Result<Vec<KernelPersonView>, KernelReadError>`、
  `latest_trace(id) -> Result<Option<&DecisionTrace>, KernelReadError>`；无效 ID 与 Faulted
  是不同结果。异常不以空列表/None/default 伪装为正常世界。
- `RenderSnapshot::from_kernel(&WorldKernel) -> Result<RenderSnapshot, RenderError>`；
  失败返回包含完整边界信息的错误。已有正常调用方显式适配，不用 unchecked 备用构造器。
- 维持**已接受 ADR-0022 的 due-instant round 预算单位**，本次不引入逐 item 可暂停调度。
  文档不再把 1,024 rounds 写成 1,024 items 或保证实时响应上限；单 round 仍随人口变化。
  `advance_to(_, 0)` 明确 InvalidBudget；`KernelConfig::new` 改为 Result，拒绝零默认预算
  或零事件容量，Default 保持有效。该验证和单位澄清计入本任务，不等到下个 Task 补问。

### D4 — 懒更新只有一个时间基线

- ActionRuntime 内 `last_needs_at` 仍是权威 materialization 基线，读视图通过只读 helper
  计算 `stored_needs.advance(now - last_needs_at)`，不重复增长、不写回、不调度新工作。
- 初始化尚无动作的 Person 以合法 Setup epoch 为基线。不能用调用方自填的当前时间、
  固定一秒增量或第二套 Needs 状态替代。
- 决策 driver 在 start/retry 等路径也须以请求 instant 的 Needs 选择，避免先用旧值
  选择、随后 start 才 materialize；继续使用原 `candidate_actions/select_action`。
- Needs 饱和规则、增长率 1/2、relief、Utility 权重、动作默认时间均不改。

### D5 — 事件先计数和摘要，后轮换

- 每个成功提交的高层 outcome EventRecord，先 validate、计数并更新顺序摘要，
  再进入容量 4,096 的诊断缓冲。摘要为有界状态，不保存完整历史 Vec。
- 推荐继续保留现有 action/kernel 两层缓冲，ActionRuntime 增加累计 total/digest；
  Kernel 在完整 instant 后取累计统计和 upstream-drop 增量，再处理自己的保留缓冲。
  `events_rotated` 包括两层实际丢失且每条只计一次；不能用 surviving Vec 长度推算总数。
- 摘要固定为有版本的非加密 FNV-1a-64 流式校验：对每条规范字段顺序的事件 JSON UTF-8，
  先纳入 little-endian u64 字节长度，再纳入正文；metadata 必须稳定排序。
  offset basis `14695981039346656037`、prime `1099511628211`；摘要乘法明确 wrapping。
  这是确定性诊断，不是防篡改/无碰撞证明；其他真实计数不得借此 silently wrap。
- total/digest 不受 drain 频率、缓冲容量和 advance 分段影响；Faulted 未完成 instant
  的统计不暴露成完整 Kernel 统计。EventRecord schema、持久化和 retention 策略不改。

### D6 — DTO 补全和显式版本变更

- `RENDER_SCHEMA_VERSION` 从 1 升至 **2**，明确拒绝 schema 1；Phase 1 transient DTO
  不承诺旧诊断 JSON 兼容，不创建迁移器，不修改 SQLite/存档 snapshot schema。
- 新增静态 `ActivitySiteRender { coord: LocalCoord, kind: SiteKind }` batch；从 Kernel
  实际 ActivitySites 读取，按 `(y,x)` 排序、坐标唯一。复用现有 `sites_of`/访问器，
  不给站点虚构 EntityId、不添加库存、经济、动态站点编辑或新的地图真值。
- `PersonRender` 增加该快照 instant 的只读 `Needs`；保留稳定 EntityId、tile、action/
  target/state。RenderMetrics 补现有可观测的 live_actions、rounds/transitions/decisions
  总数；定义沿用 Kernel，不把 Idle 等待误标为未执行，不添加未测量的 FPS/RSS 零值。
- 完整 builder 和诊断 decode 共用结构校验。width/height 必须分别为 128，cells 恰为
  16,384；先检查维度，不计算恶意大维度乘积。保留 ID 非零/唯一/排序、数量一致、
  LocalCoord/Needs 现有校验；新增站点边界、重复/排序/可行走校验。
- action/state/target 必须对应：Idle/Idle/None；Move 或活动的 Moving 同 kind 且有目标；
  Eating/Sleeping/Working 分别对应 Eat/Sleep/Work 且有目标。不能静默修正非法 wire。
- 任何公开可独立 Deserialize 的 batch/person DTO 也执行自身可检查的不变量；
  跨 batch 对应关系由根 DTO 校验。不新增“从 JSON 恢复世界/执行动作”的路径。

## 4. DAG / 执行顺序

```text
KFIX-001 拒绝原子性 → KFIX-002 同刻决策 → KFIX-003 Kernel 边界
                                             ↓
                                      KFIX-004 Needs 投影
                                             ↓
                                      KFIX-005 事件计数
                                             ↓
                                      KFIX-006 DTO 补全
                                             ↓
                                      KFIX-007 性能证据
                                             ↓
                                      KFIX-008 验收与交接
```

默认串行，因为 actions/kernel/render/exports 和回归 fixture 互有关联。
不要为了“多 agent”制造共享文件冲突。若另有明确的 Luna 分发要求，先用
`codex-luna-dispatch` 内部评估；固定契约后的测试向量、DTO validator、CLI adapter
可以成为独占文件的叶子，主代理负责时间/失败/事件语义、共享文件和一次独立复核。
本计划不要求使用 OpenCode；不得切换到 opencode-go 或修改用户 DS 配置。

### 共同文件与证据边界

- **实现**：各 Task 指定的 sim-core 文件及其新建同模块 private helpers；
  `lib.rs`、crate manifest 仅为本修复接线，由主代理持有。
- **实际直接调用方**：当前位于 `crates/sim-core/{src,tests,examples}`；memory tool
  通过 path module 引入 action example。当前 Godot/runner 仍用 Phase 0 API，未调用
  新 Kernel/DTO；不借适配之名实施 030/031/032。
- **配套文档**：本文件、修复 prompt、新修复 ADR；原 ADR-0021/22/23 仅添加后继链接；
  `docs/tasks/CHRON-027.md`–`CHRON-029.md`、其 3 份报告、
  `docs/{ARCHITECTURE,PERFORMANCE,PHASE_1_PLAN,PHASE_1_REMAINING_EXECUTION}.md`
  仅同步本次契约/状态/证据，不修改其他 Task 的实施批准范围。
  027–029 的既有批准按原记录逐项保留；本修复获批不意味着将 P1-REMAINING 全文
  或 CHRON-030–036 批量标为 Approved。
- **修复产物（待实施时创建）**：`docs/reports/P1_KERNEL_REPAIR_V1.md`，
  `docs/reports/data/kfix-v1-*.jsonl`，最终 `docs/CURRENT_PROGRESS.md`。
  8 个 Task 共用报告的 8 个 section，不新增 8 份重复审批/验收文书。
- **依赖/工具**：仅 KFIX-001 必要的 Scheduler capacity 预检；KFIX-007 的 benchmark
  examples、`tools/bench-memory/{src/main.rs,tests/cli.rs,README.md,Cargo.toml}` 和必要
  manifest/lock 接线，只复用锁定依赖。`src/rss.rs`/native unsafe 边界不在修改范围。
- **禁止改动**：Master Spec、原始历史测量数据、018–026 无关实现、Utility 默认表、
  地图/寻路算法、ECS 身份/存储格式、Godot 客户端、全局 skills/provider 配置、CI 保护设置。
  不恢复已由用户授权移除的 dependency-direction 测试，不顺手重构。

## 5. KFIX-001 — 被拒绝动作不改变世界

### Context / Objective

修复 F02a/F02b，使 start/cancel 的拒绝路径真正无副作用。

### Scope

按 D1 预检身份、当前动作、时间、目标/path、需要的 follow-up due/token/EventId；
再提交该人物修改。时间检查覆盖 last materialization 和更晚的已执行动作边界。
保留 pending retry、旧 check token 直到新操作已可提交。

### Out of Scope

不同人物事务、全世界 rollback、改变权重/地图/动作时长；不实现 Kernel 故障恢复。

### Dependencies

§1/2 当前实现及复现；ADR-0003/0004/0019/0021。主代理在首个源码修改前记录修复 ADR
和工作区源码清单；保留旧 benchmark 构建产物/数据，供 KFIX-007 同场景比较。

### Files Modified / Allowed

`crates/sim-core/src/actions.rs`、上述 private helpers、`tests/kernel_repair.rs`/公共 fixture；
必要时 `crates/sim-scheduler/src/lib.rs` 及其单元测试，只增加 D1 预检；共同文档/exports。

### API Contract

正常 start/cancel 签名尽量保持；增加 typed 时间/预检错误时在 ADR 和全部 match 调用方同步。
Rejected 后 Person/Needs/action、逻辑 token、queue metrics、事件、计数和后续执行轨迹不变。

### Execution Steps

先把两例加入 Cargo 回归测试并确认旧实现失败；实现 prepared/preflight；再覆盖 pending retry、
Unreachable、重复 cancel、UnknownPerson、AlreadyExecuting、近时间/编号上界的同类拒绝路径。

### Tests

`cargo test --locked -p palimpsest-sim-core --test kernel_repair kfix_001_`（待新增）；
`cargo test --locked -p palimpsest-sim-core --lib actions::tests`；如改 Scheduler，另跑其 crate 测试。
不只断言 Err：比较前后状态，并推进时间确认旧动作/retry 仍执行一次、新事件不凭空产生。

### Benchmark

受影响 action 100/1,000 的正式 timing/RSS 由 KFIX-007 对最终源码统一执行；本 Task 先通过
局部正确性门。无独立性能通过声明，也不将 027 的旧测量直接算为新源码结果。

### Definition of Done

F02a/F02b 和同类拒绝回归通过；无完整世界复制/新执行语义；必要 Scheduler 预检有
0/1/2 次申请、token/order 极值和无队列修改测试；报告记录实际命令和改动。

## 6. KFIX-002 — 合并同人同刻决策请求

### Context / Objective

修复 F01 的双 start 根因，让合法同刻 completion/check 正常推进。

### Scope

实现 D2 的归并及共同 driver 接入，保留原始 due work/outcome FIFO、真实完成奖励和 trace。

### Out of Scope

更改 Scheduler 优先级、通过微调 44,999 或默认时长躲避碰撞、忽略 AlreadyExecuting、
替换 selector、引入每秒扫描；Kernel 的异常封锁由 KFIX-003 负责。

### Dependencies

KFIX-001；修复 ADR 的 D2 契约。

### Files Modified / Allowed

`actions.rs`/必要 private helper、`kernel.rs` 的 driver 接入、`tests/kernel_repair.rs`、
`tests/action_closed_loop.rs`、`examples/action_execution_bench.rs` 的同一 driver 适配；共同文档。

### API Contract

同一 `(person, instant)` 最多一轮选择/启动；不同人物或 instant 不合并。若提供新 batch
helper，在 `lib.rs` 导出并记录 ADR，原单请求 helper 不能继续被批处理路径误用。

### Execution Steps

先复现 §2 的 45,000 秒场景；在共享位置实现归并；接入 Kernel 和 reference driver；
检查同刻 Sleep/Eat completion 的 relief 在重选之前生效，而非使用先到 check 的旧判断。

### Tests

`cargo test --locked -p palimpsest-sim-core --test kernel_repair kfix_002_`（待新增）；
`cargo test --locked -p palimpsest-sim-core --test action_closed_loop`。
覆盖 Work completion/check、Sleep relief/check、多个 Person 同刻、仅 critical 重新当选、
不同 instant 不合并；长 advance 与预算 1 分段到同一终点有相同状态/事件序列。

### Benchmark

统一 KFIX-007；不增加每人物每秒 selector，也不以减少真实完成次数换吞吐。

### Definition of Done

45,000 秒场景无 Err，Work 完成/后续启动各一次，无额外中断奖励；其他闭环回归不退化；
两个 driver 共用规则，不靠 catch Err 掩盖；实际命令写入共用报告。

## 7. KFIX-003 — Kernel 生命周期与完整边界

### Context / Objective

修复 F01 的“失败后半提交可见”和 F05 的过去调度；后续 Worker 只能读完整边界。

### Scope

实施 D3 的 Setup/Running/Faulted、初始化检查、错误上下文、动态读 guard 和 fallible
snapshot builder；补零预算/零容量拒绝。每个成功 round 即累积完整计数，不能到整个调用
末尾才记账导致前面成功 round 在后续 Err 时丢失。

### Out of Scope

自动恢复、panic catcher、通用事务、worker/线程、逐 item budget、运行中 spawn、持久化。

### Dependencies

KFIX-002；修复 ADR 的 D3；当前 028/029 直接调用方已核对。

### Files Modified / Allowed

`kernel.rs`、`render.rs` 的 guard/Result 接口、`lib.rs`；sim-core tests/examples 中所有
直接调用方的机械适配；必要的私有 cfg(test) 故障注入；共同文档。

### API Contract

严格落实 D3 的 Result 读 API、typed lifecycle/config/error 信息。
`now()`、已完成事件/计数不得冒充失败 instant 已提交；Faulted 动态读取显式失败。
正常 world/map/Needs 不因这些错误变成第二份可写真值。

### Execution Steps

先完成 lifecycle/config 校验及测试；再完成 round 提交与 Faulted guard；适配 builder 和
全部当前调用方。用合法极大 duration 导致的 checked-add 失败，或只在 cfg(test) 内的
确定性注入，在一个此前已有完整边界、且失败 instant 已处理部分人物的场景验证。

### Tests

`cargo test --locked -p palimpsest-sim-core --test kernel_repair kfix_003_`（待新增）；
`cargo test --locked -p palimpsest-sim-core --test kernel --test render`。
矩阵：未来/过去/重复 start、Setup 非空 advance、空世界 advance、post-start spawn、
零预算/config、正常 equal/regression、成功一轮后下一轮失败、Faulted 重复变更/读取。
快照及动态读必须 Err；last_complete/failed_at/cause 正确；前一完整边界统计不丢失。
原 F05 序列按 D3 在非法正向 advance 即拒绝，并另测空世界 advance(100) 后 start(0) 被拒绝。

### Benchmark

Kernel 100 人年度、action 回归与 snapshot 构造成本由 KFIX-007 统一测；无额外世界复制。

### Definition of Done

所有公开动态读取没有 Faulted 旁路；错误不会被返回为空世界或成功目标；所有调用方编译，
正常 empty/equal/segmented 行为仍正确；初始化约束与 Result API 变化记录在 ADR/报告。

## 8. KFIX-004 — 当前时刻的 Needs 投影

### Context / Objective

修复 F03，使读视图和选择上下文的 Needs 与同一个 committed/request instant 一致。

### Scope

实现 D4 的只读投影 helper，接入 `person/persons` 与 start/retry 选择上下文。
保持 `last_needs_at` 的单一权威，不为显示读调用 materialize。

### Out of Scope

改变 Needs domain、速率/阈值、Utility 默认表、活动 relief、提前制作 UI。

### Dependencies

KFIX-003 的读 guard、KFIX-001 的 preflight；ADR-0013/0018/0021。

### Files Modified / Allowed

`actions.rs`、`kernel.rs`、`tests/kernel_repair.rs`；仅必要 private helpers/共同 exports 文档。
不修改 `sim-ai/src/needs.rs` 或 `utility.rs` 来迁就投影。

### API Contract

返回的 Needs 对应 Kernel.now；同一世界反复读取无副作用，不改变下一次 due 或后续事件。
未来 materialize 仍从原基线计算一次；请求早于基线返回明确错误，不负向增长。

### Execution Steps

以现有 `Needs::advance` 建 helper；复用实际 materialization 时间；接入所有已列读路径和
决策路径；增加有/无中间读取的对照执行，不独立维护“显示 Needs”缓存。

### Tests

`cargo test --locked -p palimpsest-sim-core --test kernel_repair kfix_004_`（待新增）。
至少：100 秒 -> 100/200；重复读取 100 次与不读的最终 action/events/checksum 相同；
完成 relief 后及其后 1 秒的值；饱和值；retry 时间跨越选择阈值时 trace 使用投影输入。
在模块内断言读前后 stored Needs/last_needs_at/scheduler 未修改。

### Benchmark

由 KFIX-007 计入 Kernel/RenderSnapshot，不引入 per-second full-person 更新。

### Definition of Done

精确数值与无副作用对照均通过；未来增长无重复累计；无第二份可写 Needs 真值；
KFIX-006 能直接复用此读视图，报告覆盖 F03。

## 9. KFIX-005 — 有界事件缓冲的完整统计

### Context / Objective

修复 F04，让缓冲轮换只影响可保留的诊断记录，不损坏 total/digest/drop 统计。

### Scope

按 D5 增加 ActionRuntime 累计事件统计，Kernel 完整边界接收累计值与两级 drop 增量；
为每条事件建立稳定流式摘要。已完成事件仍使用既有 EventId/EventRecord。

### Out of Scope

扩大 4,096 默认容量、保存全年 Vec、数据库/Event Store 新接入、history retention、
全世界 canonical truth hash（留在 CHRON-032）。

### Dependencies

KFIX-003 的完整边界统计、KFIX-004 的确定状态；修复 ADR D5。

### Files Modified / Allowed

`actions.rs`、`kernel.rs`、必要同模块 digest helper、`lib.rs`、`tests/kernel_repair.rs`、共同文档。
不修改 `sim-events` schema 或 storage。

### API Contract

Kernel events_total 指该 Kernel 完整边界上生成的全部高层事件，不是 buffer.len。
events_rotated 指两级缓冲累计实际丢失数量；主动 drain 不计丢失。
满足 `total = 已主动交付数 + 当前保留数 + 实际轮换丢失数`；摘要不依赖 drain/容量。

### Execution Steps

先固定 digest 编码并写小向量 oracle；补上游 pre-rotation 累计值；在 round commit 发布，
与 Kernel 缓冲 drop 分开计算后汇总；保留正确的 EventId 顺序与有界存储。

### Tests

`cargo test --locked -p palimpsest-sim-core --test kernel_repair kfix_005_`（待新增）。
4,095/4,096/4,097 人同刻完成；容量 1 和默认容量；无 drain/每 round drain；
advance 分段；summary 同序/逆序/改变一条 payload；Faulted 不发布半轮摘要。
4,097 例必须 total=4,097、buffer=4,096、rotated=1。
摘要用独立小向量期望验证，不把被测函数自身输出当 expected；不声称摘要抗碰撞。

### Benchmark

KFIX-007 测新增每事件统计成本和 RSS；本 Task 的 4,097 边界测试不是正式规模性能 gate。

### Definition of Done

容量边界、计数恒等式、摘要/分段不变性通过；不丢计、不重复计、不增加无界历史；
Kernel 的 event metrics 含义与报告一致。

## 10. KFIX-006 — 完整且可验证的 Render DTO

### Context / Objective

修复 F06 和 G01 的字段缺失；让 029 为未来显示提供真实、完整的 schema 2 只读数据。

### Scope

实施 D6：ActivitySite/Needs、已有 metrics 补齐、schema 2、维度和对应关系验证；
保留 KFIX-003 的完整边界 Result builder。

### Out of Scope

Godot 转换/场景/Worker、存档迁移、兼容 schema 1、新增站点/人口玩法或 UI 计算真值。

### Dependencies

KFIX-003/004/005 的读 guard、Needs、事件统计；修复 ADR D6。

### Files Modified / Allowed

`render.rs`、`kernel.rs` 的只读站点出口、`lib.rs`、`tests/render.rs`、
`tests/kernel_repair.rs`、`examples/render_snapshot_bench.rs` 的 API 适配；共同文档。
站点枚举用现有 `ActivitySites::sites_of`，无需新增 sim-world 模块公共接口。

### API Contract

D6 的精确字段与 schema 2；根 builder/decode 均验证，独立 DTO decode 没有自身不变量旁路。
站点不由 Godot 猜测，Person Needs 等于该快照 instant 的 Kernel 只读投影；无 ECS/token。

### Execution Steps

先补字段/版本/访问器，再实现共享 validator；适配所有正常 builder 调用方；从真实
快照逐项污染 wire，增加对应负例。保留旧版本拒绝，不自动升级/修正导入内容。

### Tests

`cargo test --locked -p palimpsest-sim-core --test kernel_repair kfix_006_`（待新增）；
`cargo test --locked -p palimpsest-sim-core --test render`。
矩阵：width/height 0、127、129、usize 极值；cells 错数；0/重复/乱序 Person ID；
坏坐标/Needs；站点重复/乱序/不可行走；action-state-target 冲突；schema 1/未知版本；
空世界合法快照；100 人+真实站点逐字段一致；重复 build 不修改 world；Faulted builder 失败。
原有非法 ID/数量测试仍须保留；因新 Result/schema 的机械适配不构成删除断言许可。

### Benchmark

KFIX-007 测完整 schema 2 的 build/serialize/bytes/RSS，而不是裁掉新字段后测试。

### Definition of Done

F06 被拒绝、G01 字段真实齐全、schema 2 的全部正负例通过；所有调用方编译，
无独立反序列化旁路；不提前开发客户端，不承诺旧诊断格式兼容。

## 11. KFIX-007 — 补齐正式测量与工具

### Context / Objective

关闭 G02，为**修复后源码**补足 027 回归、028 年度吞吐及 029 schema 2 的正式性能证据。

### Scope

已有 action/kernel/render 三个 example，但后两者缺两次完整 warm-up、完整 raw 协议、
RSS adapters；render 当前把全部 terrain 字节摊成 bytes/person。完善这些工具并采集。
baseline、timing、cold/prepared RSS 是不同证据，不能互相替代。

### Out of Scope

10 年 chaos、Godot/FPS/worker 比较、其他人口正式扩容、native RSS 算法/unsafe 修改、
性能预算放宽、因测量慢而缩短年度或减少人口。

### Dependencies

KFIX-001–006 已通过局部验证；ADR-0020；KFIX-001 保留的原源码身份和 baseline。
无可信可比 baseline 的项目建立首次正式基线，明确“不证明优化/未回退”，不能伪造前后比较。

### Files Modified / Allowed

`crates/sim-core/examples/{action_execution_bench,kernel_bench,render_snapshot_bench}.rs`；
其 CLI/adapter 回归测试、必要只读 counters 接入；
`tools/bench-memory/{src/main.rs,tests/cli.rs,README.md,Cargo.toml}`；必要锁定依赖接线；
共用修复报告/新 raw、027–029 报告的追加修复证据与 PERFORMANCE 索引。
必要新脚本仅在 `tools/` 下用于本计划数据采集，不创建常驻监控/服务。

### API Contract

- kernel/render examples 增加并真正实现 `--warmups N --samples N --json`；action 也支持
  该 JSON raw 选项，原合法参数和正确性断言保留。命令行非法/缺值/零正式样本显式失败。
- JSON 输出记录 fixture/seed/config、units、warmups、每样本时间/校验/统计、
  min/median/max；中位数统一排序后上中位（index n/2），明确与旧 pilot 算法的差别。
  采集时 stdout 保持 JSON，诊断走 stderr；不能只输出最后一次值冒充整个样本集。
- 增加 memory selectors：`kernel-100-year`、`render-control-100`、`render-snapshot-100`；
  沿用 `--run <case> 3`、新冷进程、两次 observe、结果存活期和 proof 规则。
  列表保留原 24 项并加入 3 项（没有其他新增 case 时应为 27），不能用删旧 case 凑计数。
- 短 adapter/CLI 测试验证路由和协议；完整年度只在显式 benchmark 执行，不把新年度测量
  塞进每次 unit test，也不将已有测试改为 ignored/skipped。

### Execution Steps / Measurement Contract

1. 先完成采集能力和正确性测试，再冻结本轮测量源码。若保留旧 binary，用其**实际支持**
   的 flags 测同场景，记录其 source/config；不要将新增 flags 传给旧 parser 误称生效。
   修改输出协议不得改旧 fixture/workload 以制造“同场景”比较。
2. 核实 M5/16GB、OS/toolchain/Cargo.lock/相关源码 hash，报告当前仍为 dirty 工作区。
3. 先用 100 人一天 smoke 估算完整年度的墙钟/资源成本，再执行完整测量。正常长跑不是
   新审批门；后台命令保持进度可见。异常保留错误/部分证据，修复范围内原因后重新完整采集。
4. 无并发构建、测试或其他 agent 重负载。timing release、2 次完整工作负载 warm-up、
   10 次正式样本，校验不得关闭；RSS 每 case 3 个独立冷进程，单独记录。

| 测量 | fixture / 时间区间 | 必须输出 |
|---|---|---|
| Action 100/1,000 | 原 seed 25,025 分散出生 fixture；172,800 秒；setup 在计时外，初始决策和闭环在计时内 | wall min/median/max、transitions/wall-s、sim-s/wall-s、完成数、checksum、队列健康 |
| Kernel 100 | 保留原 seed 42 默认站点与首个 walkable 出生点；先 86,400 秒，再 **31,536,000 秒**；setup/start 不计入 advance 时间 | wall、sim-s/wall-s、rounds/s 与 transitions/decisions、events/wall-s、event total/digest、观测到的最大队列深度 |
| Render 100 | 同 seed 42，Kernel 先实际到 600 秒；完整 schema 2；每次真实 build 和 serialize | 各自耗时、total bytes、terrain/sites/persons/metrics 子段字节数、persons-array bytes/person |
| action-100/1000 RSS | 既有 86,400 秒 adapter，修复后再测 | 总 RSS high-water、cold 与 prepared 的独立 proof/增量 |
| kernel-100-year RSS | 同 Kernel timing fixture，100 人/31,536,000 秒，Kernel 存活至第二次 observe | 同上；不使用旧 action RSS 当 Kernel RSS |
| render-control-100 RSS | 与 render 相同的 600 秒已准备 Kernel，执行相同只读验证但不创建 DTO；Kernel 保持存活 | 单独 control cold/prepared；不是另一个世界/人口 |
| render-snapshot-100 RSS | 同 control 准备状态，build+serialize，并保持 Kernel/DTO/字节缓冲至第二次 observe | 单独 snapshot cold/prepared；不相减两个进程 lifetime peaks 冒充 operation peak |

prepared interval 被早前峰值遮蔽时，按 ADR-0020 原样记录 unavailable/proof，仍交付可证明
cold interval；这不是工具缺失的 N/A。cold interval 无法证明时不算完成，也不能偷偷重试筛选。
bytes/person 用 persons 数组的序列化长度/N（说明含数组括号/分隔符），不包含 terrain/sites；
整包另报。JSON 子段长度之和未必等于整包长度，单列 envelope/分隔开销，不伪造精确堆大小。
render 的空人口功能测试应通过；计费式每人指标 N=0 时 unavailable，不除零。

### Tests / 实施时待补齐的命令

下列 example 存在，但 `--json`、后两者的 `--warmups` 及 3 个 memory selectors
**待本 Task 创建**。不是当前已运行证据。

```sh
cargo test --locked -p palimpsest-sim-core --examples
cargo test --release --locked -p palimpsest-bench-memory --test cli
cargo build --release --locked -p palimpsest-sim-core --examples
cargo build --release --locked -p palimpsest-bench-memory

target/release/examples/action_execution_bench --persons 100 --seconds 172800 --warmups 2 --samples 10 --json
target/release/examples/action_execution_bench --persons 1000 --seconds 172800 --warmups 2 --samples 10 --json
target/release/examples/kernel_bench --persons 100 --seconds 86400 --warmups 0 --samples 1 --json
target/release/examples/kernel_bench --persons 100 --seconds 31536000 --warmups 2 --samples 10 --json
target/release/examples/render_snapshot_bench --persons 100 --warmups 2 --samples 10 --json

target/release/palimpsest-bench-memory --run action-100 3
target/release/palimpsest-bench-memory --run action-1000 3
target/release/palimpsest-bench-memory --run kernel-100-year 3
target/release/palimpsest-bench-memory --run render-control-100 3
target/release/palimpsest-bench-memory --run render-snapshot-100 3
```

每个样本都断言目标真正 Reached、人数/身份/需求合法、预期动作真实完成、事件摘要/计数
在同输入样本间一致；render 逐字段/字节校验。golden 若因已批准修复发生合理变化，
先解释首处分歧并独立核算新固定 expected，保留旧证据；禁止测试运行时自写/自学 expected。

### Benchmark / Evidence

原始 timing 保存 `docs/reports/data/kfix-v1-{action,kernel,render}-timing.jsonl`；
memory 保存 `docs/reports/data/kfix-v1-memory.jsonl`；before 数据另用 `kfix-v1-before-*`。
保留失败/不完整记录并标注，历史 `chron-027/028/029-*` raw 不覆盖、不重新标成正式样本。
没有吞吐硬下限或快照字节新预算，不发明 pass 阈值；已有 3/5/7GB 与 60 FPS 产品目标不变，
本 Task 不声称验证客户端 FPS 或完整 Core+Client RSS。

### Definition of Done

完整 workload 的 2+10 timing 与 3 cold RSS 证据齐全，adapter/CLI tests 通过；
缺测项不推给产品负责人/CHRON-033 后宣布本次完成；报告区分正式结果、pilot、
不可比 baseline、prepared 不可证明项及剩余产品性能目标。

## 12. KFIX-008 — 一次整体验收与当前进度

### Context / Objective

闭环 G03：以最终源码证明修复完成，让下一聊天无需依赖本次会话或临时文件。

### Scope

一次主代理跨文件复核、一次最终质量门，统一修复报告和 `docs/CURRENT_PROGRESS.md`；
同步 027–029 的当前状态、修复 ADR、相关计划/架构/性能索引。

### Out of Scope

重新审计未变化的 018–026、另起多轮 reviewer、远端 CI/PR 操作、新聊天创建、
CHRON-030+ 实现、Phase 1/Phase 2 总体验收结论。

### Dependencies

KFIX-001–007 的局部/正式证据和最终文件清单；共同源码身份。

### Files Modified / Allowed

共同文档、修复报告、`docs/CURRENT_PROGRESS.md`。本 Task 本身不顺手改代码：
若验收发现同范围 bug，回到负责的 KFIX Task 修复，再按受影响程度复验，不请求重复批准。

### API Contract

N/A，无新运行时 API。进度文件是执行索引，不替代 Master Spec/ADR，也不能把 Approved
或 Implemented 自动写成 Verified。历史“当时批准”不删除，但当前验收状态要有日期。

### Tests / 最终统一门

```sh
./tools/ci-rust.sh
cargo test --release --locked --workspace --all-targets --all-features
cargo test --locked --workspace --doc
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
cargo metadata --locked --no-deps --format-version 1
cargo tree --locked --workspace --edges normal
./tools/ci-godot.sh
git diff --check
shasum -a 256 MASTER_SPEC.md
```

ci-rust 已含 fmt、workspace Clippy -D warnings、debug tests、MSRV 和旧 smoke，
不再把同命令抄成额外一整轮门禁。KFIX-007 最终源码的 native CLI 结果复用，
其后该调用链有变化才重跑。Godot 只做**现有 Phase 0 smoke**，不声称完成 030/031/FPS。
核对 exact normal dependency graph；不声称已移除的自动审计还在执行。

### Benchmark

复用 KFIX-007 最终源码/配置的证据，纯文档变化不再重跑十样本。若修复改变 measured
runtime/fixture/collector，重跑受影响测量并更新 source identity；不重测无关旧模块。

### Required Completion Report

修复报告用 F01–G03 映射记录：实际文件、确切命令/测试数、前后结果、DoD 证据、
benchmark 路径、敏感变化、限制。以下信息写入 `docs/CURRENT_PROGRESS.md`：

- 更新时间、分支/HEAD/dirty 状态；未跟踪源码必须随工作区交接，不能只拉远端旧 HEAD。
- 027/028/029 的 code/test/performance 分栏状态，KFIX-001–008 状态和唯一报告链接。
- 完整验证命令、结果和 source identity；哪些是本地测试，hosted CI 当前未验证。
- schema 2、Result API、Setup/Faulted、拒绝行为、round 预算含义等敏感变更。
- 未完成/阻断事项；Phase 1 的 100 NPC/10 年尚属 032，不因年度 benchmark 而标为完成。
- 下一候选 Task 与依赖，只提供建议，不自动执行；新聊天应先读进度和对应 Task。

### Definition of Done

F01–G03 均有证据闭环；最终检查通过且 Master hash 不变；没有修改 native unsafe、
游戏内容/权重/存储/远端设置；CURRENT_PROGRESS 可独立接续且无“等用户另行跑 benchmark”
隐藏工作；向用户报告敏感变化与剩余限制，然后在本计划终点停止。

## 13. 轻量审查、暂停条件和本轮文档交付

### 精简排期，不删质量要求

- KFIX-001–006：编写并跑受影响回归 + 局部 fmt/lint/编译，完成时自检本 Task diff，
  不另起 reviewer 流程；修复失败项后重跑相关项。新 `kfix_NNN_` filter 必须实际发现
  预期用例，0 tests 不算通过。若使用子代理，主代理接收其 patch 时做必要集成检查，
  不再叠加第二、第三轮同质审查。
- KFIX-007：所有实现整合后统一测量一次；KFIX-008：统一全量验证和父代理复核一次。
  后续无关文档修改可复用证据；不安排“审查完成后再审查审查过程”。
- 这份计划被执行批准后，上述排期替代 027–029 在每个内部修复步骤重复全套验证的要求；
  测试本身、正式样本数、预算和最终 workspace/MSRV/Godot smoke 均未删除或降低。
- 暂停只用于 Master 冲突、确需改变本文已定语义/范围/外部权限的增量，或穷尽范围内方案
  仍无法解决的真实阻塞。通常测试失败、已列文件新增 helper、ADR 和配套工具不是审批原因。
  Master 冲突写 CP；不把普通 Task/ADR 同步当成 CP 审批单。

### 当前“编写计划”Task Contract

- **Context**：用户要求把上轮审查问题写成可执行修改任务并提供修复 prompt。
- **Scope**：本修复计划、配套 prompt；明确任务/决定/文件/API/证据和精简审查排期。
- **Out of Scope**：实际修复、修改既有 Task 的当前批准状态、运行正式测量、创建新聊天。
- **Dependencies**：必读文档、027–029 Task/ADR、当前调用链和上轮 7 个复现证据。
- **Files Modified / Allowed**：本文件、`docs/prompts/P1_KERNEL_REPAIR_V1.md`。
- **API Contract**：本文仅记录建议，未实施任何 API/版本/行为变化。
- **Tests**：Markdown 链接/代码块/必需 Task 字段、F01–G03 覆盖和 DAG、命令入口存在或
  明确标为待建、`git diff --check`、既有文件哈希不变验证。
- **Benchmark**：N/A，纯文档不产生运行时性能结果。
- **Definition of Done**：8 个 Task 目的单一、范围/依赖/验收齐全、prompt 无隐藏扩展，
  仅两份文档新增，Master Spec 和所有既有文件保持不变；向用户提供链接和修复 prompt。
