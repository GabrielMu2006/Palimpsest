# Phase 1 后续执行补充计划

- **Plan ID**：P1-REMAINING；**Revision**：2026-08-30-r1。
- **Task 集合**：CHRON-027–036；已有 018–026 和 REM-008A 是依赖，不重做。
- **状态**：Proposed，本文是计划澄清，尚未批准 CHRON-027+ 实现。
- **批准方式**：用户明确要求执行本计划，即接受本文的推荐决策和配套步骤；
  不再逐项批准。内部设计、ADR 落字、Luna 能力判断不另设用户审批。
- **推荐的外部操作范围**：027–033 先本地实施；034 的 hosted CI 验收需要提交并
  push 已审查候选变更到 `GabrielMu2006/Palimpsest` 的 `codex/p1-remaining-r1`
  分支、建立或复用该分支到 `main` 的 Draft PR；035/036 在同分支更新并复验。
  执行本计划时此步骤一并批准，不在临近 CI 验收时补问。**不含 merge、直接写 main、
  force push、修改可见性/保护、修改现有其他 PR、发布版本或收费服务**。
  本次仅改文档，以上操作均未执行；不改用户 OpenCode 配置。
- **终点**：CHRON-036 报告交付并等待 Phase 1 验收；不进入 Phase 2。

本文与 [Phase 1 总计划](PHASE_1_PLAN.md)、[执行契约](EXECUTION_CONTRACT.md)、
各 Task 一起使用；总计划中的 018–026 摘要保留为规划背景，实际接口以已验收
源码/ADR 为准。本文的新增实现决策仍是 Proposed，不能当作已运行的结果。

## 1. 已核对的起点

2026-08-30 的工作区包含已完成但未提交的整改；执行前记录 HEAD 和 dirty diff，
保留这些修改，不把旧 HEAD 当成最新源码。

| 已有能力 | 后续必须使用的真实边界 |
|---|---|
| `PersonRuntime` | 通过 `EntityId` 读写 Location/Needs；没有现成 CurrentAction 或 kernel；不要暴露 ECS handle |
| `Needs::advance/eat/rest` | 整数压力，增长 raw 1/2 每秒；已有满足需求方法，不新建经济系统 |
| `candidate_actions` / `select_action` | Work 默认权重 2300（ADR-0018）；三个构造器已改 Result；结构合法不代表目标仍可执行（ADR-0019） |
| `find_path` | 返回 `Result<Path, PathError>`，包含起终点，默认预算 16,384；不是无错误的 `Vec`；已有确定性 tie-break 不改 |
| `Scheduler<T>` | due-time/FIFO；取消使 live entry 失效，允许 lazy stale nodes；用既有 compact/metrics 验证界限 |
| `EventRecord` | 已有字符串 event_type 和 metadata；优先组装有效记录，不为动作另改事件 crate/schema |
| `tools/bench-memory` | 能测 019–026；027+ 需要新增用例，native RSS 逻辑沿用 ADR-0020，不能把已有结果套给新 kernel |

依据：[整改闭环](reports/PHASE_1_REVIEW_REMEDIATION_PLAN_V1.md)、
[内存工具验收](reports/REM-008A_MEMORY.md)、ADR-0013–0020 及当前源码。
短 benchmark 不代表 100 NPC/10 年闭环已通过。

## 2. 推荐实施决策（执行本计划时一起接受）

以下把可预见选择提前列出。主代理将这些语义及具体 Rust 签名写入 Task 对应 ADR
**后再写实现**；ADR 编号从当时未占用编号分配，不修改已接受决定的历史内容。
参数是 Phase 1 验证用默认值，不是最终玩法平衡或新性能预算。

### D1 — 动作和需求闭环（027）

- `sim-core` 拥有动作状态；`sim-ai` 继续只计算候选/分数，不接收执行副作用。
  执行器接收同一 live context 产生的选择，不接受导入的诊断 JSON 作为命令。
- 默认移动每相邻四方向 Tile 用 1 个模拟秒；路径包含起点，不能把起点再走一步。
  不做 NPC 碰撞/占位互斥，允许多人同 Tile；不新增动态地图编辑或重新寻路系统。
- Eat/Sleep/Work 可以包含前置移动阶段，到目标后才进入活动阶段；独立 Move
  到达即完成。统计分别记录“移动阶段完成”与“顶层动作完成”，不得伪造 Move 选择。
- 默认 Eat 600 秒，Sleep 28,800 秒，Work 1,800 秒，Idle 等待 60 秒。
  Eat/Sleep 成功完成后，分别通过现有方法减少 100,000 raw hunger/fatigue
  （clamp 到零）；Work 只递增既有有界观察计数。中断/失败不给完成奖励。
- 需求按真实 elapsed 模拟秒更新一次，不重复累计；完成时先累计到该 instant，
  再满足需求。保留 ADR-0018 默认权重和增长率，不靠改权重让闭环测试通过。
- 普通动作完成再决策；每个 Person 另有下一次 critical need 边界检查。
  到该边界用原 selector 比较；只有另一 action/target 胜出才中断，保留完整 trace，
  不新增绕过 Utility 的“紧急随机行动”。同 instant 仍按 Scheduler FIFO。
- 启动非法/重叠动作不改变现有动作；执行中 blocked/failed 取消关联 live token，
  回 Idle，最早下一模拟秒再尝试，禁止同 instant 无限重试。已在 critical 状态的
  重查也必须有正延迟，默认 60 秒；动作取消和 stale token 都有重复调用测试。
- ADR-0018 的真实闭环测试在 027 内用小型测试 driver 完成，不反向依赖 028：
  seed 25,025 的既有可达 fixture，持续 172,800 秒，重复两次；断言 Work 完成、
  Eat/Sleep 完成且降低对应需求、恢复到双低需求后回 Work。另测不可达/中断路径。

### D2 — 调度、提交和容量（028）

- kernel 是唯一时间/执行所有者。直接跳到 next due instant，不以 1 秒循环扫描所有
  Person；按需累计 Needs，读视图投影到当前 instant，不把未 materialize 的旧值当当前值。
- 提供 `advance_to(target, work_budget)` 一类有界接口；结果显式区分到达目标和
  budget 用尽后 Yielded，并报告实际 committed instant。后者是可继续进度，不是假成功。
  单个 due item 原子提交；不承诺回滚已经提交的整段历史。拒绝回退目标且不修改状态。
- FIFO 记录的顺序不因 budget/调用拆分变化；同 instant 未排空前不发布快照或应用
  新命令。错误须报告最后完整边界，不能宣称已到 target。计数溢出等致命错误显式停止。
- 默认每次最多 1,024 due-instant rounds（ADR-0024/0025）；每轮可有多个items，不承诺墙钟响应上限。每 Person 最多两个 live schedule 项（动作/重试及
  need 边界），重复更新替换旧 token。验证 live 上界与既有 Scheduler stale-compaction
  界限，不把允许的 stale 节点当泄漏，也不要求持续世界终点 future queue 为空。
- 只保留每 Person 最新完整 DecisionTrace；高层动作事件流每条都验证、计数并进入
  顺序摘要，再由消费者 drain；诊断缓冲上限 4,096，缓冲轮换计数可见。不存全量
  10 年 Vec，也不声称这是永久 Event Store 或新的历史 retention 策略。
- 必须测试一次长 advance 与不同预算/分段 advance 得到相同 truth/事件摘要。
  先跑短闭环估算一年/十年成本；必要的计数器在本 Task 做，不留给报告任务猜测。

### D3 — DTO、worker 和表现（029–031）

- DTO 从 kernel 的完整边界构造，不让调用方指定另一 `now`；稳定 ID、row-major
  地图、按 ID 排序的 Person、明确 schema_version。诊断 decode 也做版本/数量/
  重复 ID/坐标验证，导入值永远不能写回世界。不建立存档兼容承诺。
- 增加静态 ActivitySite batch 和只读 Needs/动作/指标所需值，031 不自行生成站点。
  暂无指标显示 unavailable，不使用零冒充已测量；墙钟/RSS 不进 truth hash。
- 单 worker 安全 Rust，标准库有界队列默认容量 64。Full/Closed 明确返回。
  命令带顺序标识和已应用边界的确认；拒绝/队列满不能显示成操作成功。
- 速度集合固定为 1/5/20/100/1000/MAX；速度控制目标时间的 wall-clock pacing，
  不改变系统 cadence 或 LOD。MAX 无 pacing；不能以少做模拟来获得加速。
- Step 仅暂停时允许，1 step = 1 模拟秒，0 是无副作用 no-op，单次最多 1,000；
  非暂停 Step 拒绝。worker 初始暂停并提供初始快照。AdvanceTo 是仅在暂停时
  接受的测试/批处理命令，显式推进到目标后仍暂停；它仍用相同 kernel 语义，
  明确确认后才可宣称到达，不是 Godot 直接改时间。Pause 停止自动 pacing，
  不阻止随后明确提交的合法 Step/AdvanceTo。
- Pause/Shutdown 在完整 due 边界生效；队列满时关闭仍有独立停止通道/标志，
  不依赖再入队一个 Shutdown。报告真实命令响应延迟，不许承诺“绝不延迟”。
- exchange 至多两个拥有的快照，读者只保留当前帧；“latest”指读取时已经发布的
  最新完整快照，允许沿用它直到下一次发布，禁止倒退或观察半提交。
  发布目标 10 Hz wall-clock（暂停/step 强制刷新），与 simulation truth 无关。
- 031 复用 Tile renderer，bridge 每帧批量读一次；稳定 ID 用无损表示，不能经过
  f64，必须测 `u64::MAX`/超过 `i64::MAX` 的边界。移除/移动 Node 不改变 Rust truth。

### D4 — 长跑、测量、CI、退役（032–036）

- 验证年 = 365 × 86,400 秒；10 年 = 315,360,000 秒，从 epoch 开始。
  不是新日历 lore，不通过改“每年秒数”缩短权威验收。
- 032 固定 seed 42、100 Person、可达 Meal/Rest/Work；无唯一占位要求。先验证
  生成 fixture 的连通性，不能 teleport、跳过执行或手工分配动作来满足行为统计。
  每人都须有移动阶段和 Eat/Sleep/Work 完成，Idle 状态在总体中可观测。
- canonical truth hash 覆盖配置/时间/按 ID 的 Person/动作/需求/逻辑待执行工作、
  有界工作计数；事件/decision 顺序另作流式摘要。排除 wall-clock、RSS、线程身份、
  指针和 ECS handle；同 seed/config/input 才比较确定性，绝不直接 hash unordered map。
- 全程收集有界 invariant 汇总，报告按模拟天记录 checkpoint；守卫每个提交的合法性。
  watchdog 在 runner 外层报错/非零退出，不伪造 kernel 能恢复 panic 或无限循环。
  N/A 仅用于未实现的死亡、经济、数据库等系统，不用于遗漏的内存/行为证据。
- 032 做三次独立完整 10 年运行，比较确定性，报告时长 min/median/max；先做独立
  短 smoke 热身。033 做 100→1K→3K→5K→10K 的分级诊断，每级至少十次 timing；
  高级超时/超内存如实失败/未完成，不能跳过小级或降低配置伪造全套完成。
- 034 种子 corpus 为 0/1/42，CI smoke 用真实 kernel 跑 86,400 秒，不把 seconds
  谎称 years；完整十年仍属 M5 本地 gate。初始 goldens 由本 Task 独立复核生成，
  后续变动只能来自已批准的行为变更，测试不能自写期望值。
- 034 提交前逐文件审查候选 diff，包括必要且已验收的 018–026/REM-008A 未提交
  依赖；不要 `git add .` 收入无关修改。保留逐 Task 变更说明。目标分支若已存在，
  先核对归属/历史，不覆盖其他工作。PR 保持 Draft，不动现有其他 PR 或 main。
  记录候选 SHA 和两项实际 hosted checks；035/036 更新后重验最新 SHA，旧绿色无效。
- 035 只在真实替代路径、回归检查与原测试覆盖映射都齐备后退役 dummy API。
  退役不是清理无关代码；相关测试迁移到真实路径或保留独立 primitive 覆盖，
  不能删除/弱化测试来让移除编译通过。历史报告/raw artifacts 原样保留。
- 036 只能汇总已有证据。缺证据退回责任 Task 补齐，不将“代码齐了”写成通过。
  保留 3/5/7 GB 与 60 FPS 目标；本阶段不开 LLM，不能声称验证了 7 GB 整套配置。

## 3. 配套文件范围与所有权

以下是**对应 Task 的必要配套范围**，不是所有 Task 可任意改所有文件：

| Task | 实现/配套所有权（加上该 Task 原有文件清单） | 必须先固定的契约 |
|---|---|---|
| 027 | `sim-core` actions/person 接入、examples/tests；memory tool 的 action 用例 | D1，原子 transition、错误、事件 metadata；ADR |
| 028 | `sim-core` kernel/config/metrics/tests/examples；runner 的 kernel 入口 | D2，progress/partial commit、cadence、逻辑调度摘要；ADR |
| 029 | `sim-core` render DTO/builder/validation/tests/example | D3 的字段、版本/非法输入、大小方法；ADR |
| 030 | `sim-core` worker/commands/tests/example；thin bridge lifecycle 接口 | D3 的线程/背压/ack/暂停/关闭；ADR |
| 031 | `apps/macos-godot`、`godot-bridge` DTO 转换、`tools/ci-godot.sh` 和帧采集脚本 | D3 无损 ID、批量转换/控制反馈；ADR |
| 032 | `apps/headless-runner` 的 chaos API/bin/tests、`tests/worlds` fixture、kernel 只读诊断 | D4 年长、invariants、hash/摘要和 watchdog；ADR |
| 033 | runner/`benchmarks` 驱动、memory tool 新用例、Godot 帧/bridge 采集、只读 counters | 同工作负载的 direct/worker/rendered 比较，无 gameplay 优化 |
| 034 | `.github/workflows/ci.yml`、`tools/ci-*.sh`、runner tests、`tests/{worlds,simulation,regression}` | corpus/goldens、本地与 CI 命令、精确 normal dependency 人工审查 |
| 035 | 仅 spike 直接调用链的 core/runner/bridge/Godot script/tool/CI、README | 移除清单、替代覆盖清单、ADR-0010 退役记录 |
| 036 | 最终报告、Task 状态及结果索引 | 数字来源和 Phase 1 验收，不做新实现 |

027–033 的 memory adapter 允许修改 `tools/bench-memory/{src/main.rs,tests/cli.rs,README.md}`
和必要的 Cargo manifest/lock（只限既有 workspace/锁定依赖）；不修改 native unsafe
边界、不新增监控 daemon 或第三方服务。新增例子内存 adapter 与原 timing 路径分离。
需要新的仪器能力但不在本范围的，先提出最小增量，而不是偷换测量指标。

每个实现 Task 均包含：本 Task spec/report、`docs/reports/data/chron-NNN-*`、
对应 ADR 新建/增补、`docs/ARCHITECTURE.md`/`docs/PERFORMANCE.md` 的相关说明与
索引、`docs/PHASE_1_PLAN.md` 状态同步、对应 crate 的 manifest/export 接线和同模块
必要 helpers/tests。不含预算修改、无关重构或 Master Spec；正常文档同步不需要 CP。
036 不借此扩大为实现 Task。主代理独占共享 manifest/lib.rs/ADR/索引，子代理只改
已分配文件；同一 time window 的性能采集不并行。

## 4. 命令与证据责任

下表的新 target/selector **目前不存在**；由所属 Task 创建并把最终可运行命令写入
报告，不能把本表当作成功运行记录。默认统一 CLI 使用显式 flags。

| Task | 待创建入口与最小验收样例 | 必要报告 |
|---|---|---|
| 027 | `cargo run --release --locked -p palimpsest-sim-core --example action_execution_bench -- --persons 100 --seconds 172800 --samples 10`；再测 1000 | `CHRON-027_ACTION_STATE_MACHINE.md` |
| 028 | 同 package，`--example kernel_bench -- --persons 100 --seconds 31536000 --samples 10`，先跑一天 smoke | `CHRON-028_KERNEL.md` |
| 029 | 同 package，`--example render_snapshot_bench -- --persons 100 --samples 10` | `CHRON-029_RENDER_SNAPSHOT.md` |
| 030 | 同 package，`--example worker_bench -- --persons 100 --seconds 86400 --samples 10` | `CHRON-030_SIMULATION_WORKER.md` |
| 031 | 帧采集脚本以 release GDExtension 启动 windowed Godot；固定 warm-up 120 帧、至少 300 测量帧 | `CHRON-031_GODOT_MICRO_WORLD.md` |
| 032 | `cargo run --release --locked -p palimpsest-headless-runner --bin chaos_runner -- --seed 42 --persons 100 --seconds 315360000 --runs 3` | `CHRON-032_CHAOS_10YEAR.md` |
| 033 | 同 package，`--bin bench_micro_world -- --scales 100,1000,3000,5000,10000 --seconds 86400 --samples 10`；另做等量 100 人 direct/worker/rendered 对照 | `CHRON-033_SCALE_BENCHMARKS.md` |
| 034 | 通过 runner integration test 注册 root fixtures，不能把未注册的 `tests/regression/*.rs` 当成 cargo 会自动运行 | `CHRON-034_REGRESSION_CI.md` |
| 035 | `rg` 列出已知 spike API 引用、编译所有调用方、迁移前后覆盖映射、历史文件 hash | `CHRON-035_SPIKE_RETIREMENT.md` |
| 036 | 检查最终源码身份、全部上述报告/raw data/CI revision；必要重测回到原责任 Task | `PHASE_1_MICRO_WORLD_KERNEL_V1.md` |

- 027–030、033 timing：release，两次独立 warm-up、至少十次正式样本，完整 raw
  min/median/max；内存另用三次 cold subprocess，沿用 ADR-0020 proof，不能把三次
  内存采样代替十次 timing。记录总 peak RSS 与 cold/prepared delta 的各自含义。
- 长跑 RSS checkpoint 是趋势样本，不是精确瞬时峰值；总峰值采用已验证 native
  tool 包装同一 runner workload，tool 不反向成为 core 的依赖。
- 029 报 build/serialize 耗时和完整字节数，`bytes/person` 要分开固定 terrain 成本；
  有/无 DTO 对照只比较各自测量，不能相减独立 lifetime peaks 冒充 operation peak。
- 030 pacing 用短的受控 wall-clock 窗口测各速度响应，重型等量吞吐对照不按 1×
  实际等待一天。fake clock 只用于时间控制测试，不作为真实性能结果。
- 031 报 FPS min/mean/p95 **及 frame-time p95**、draw calls、VRAM、进程 RSS、
  snapshot age/转换耗时。基础 terrain draw call 和整场景 draw calls 分开；
  60 FPS/基础 tile 1 draw call 目标保留，失配如实记录且不可宣称通过。
- 033 对 direct/worker/rendered 使用同 seed/config/end instant 和真值摘要；不同
  pacing 结果单列，不拿 1×限速与 headless MAX 算架构开销。未达到终点的不是完整样本。

现有通用检查（实施时实际运行并记录，不因文档更新就声称已运行）：

```sh
./tools/ci-rust.sh
cargo test --release --locked --workspace --all-targets --all-features
cargo test --locked --workspace --doc
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
cargo metadata --locked --no-deps --format-version 1
cargo tree --locked --workspace --edges normal
./tools/ci-godot.sh
cargo test --release --locked -p palimpsest-bench-memory --test cli
```

Godot/native 项仅在对应平台运行；各命令不是互相替代。
dependency review 对照 ADR-0017 的精确 allow-set，人/agent 审查，不谎称有自动 guard。
干净 checkout 验证必须包含本轮全部候选修改；不能只 checkout 旧 HEAD 测出绿色。
远端设置只读核验；CI 按上述限定候选分支发布步骤触发并核验对应 revision。
本地通过不等于 hosted CI 已跑；发布权限真实缺失才是外部阻塞，不预先假报绿色。

## 5. DAG、代理分工和内部准备

```text
027 → 028 ┬→ 029 → 030 → 031 ┐
          └→ 032 ────────────┴→ 033 → 034 → 035 → 036
```

- 只有 029/032 具备 Task 级并行可能；共同 core/export/fixture 修改由主代理串行。
  “依赖无环”不自动等于“文件互不冲突”。所有余下 Task 按原 DAG 的直接依赖执行。
- 主代理承担 D1/D2 的设计和闭环、worker 并发/错误语义、架构取舍与最终验收；
  不把未知设计作为一个大包交给 Luna。
- 用户要求 Luna 时使用 `codex-luna-dispatch`：先内部判断能力，适合的 DTO 验证、
  已定接口的叶子实现、测试向量、bench adapter 可交给 GPT-5.6 Luna；共享文件
  不重叠，完成后主代理独立检查。能力不够直接本地处理，不增加审批轮次。
- OpenCode 不是本计划的必需依赖；只有明确选用它时才用用户已配置 DS API，
  不使用 opencode-go，不修改 provider，不因其不可用阻塞可由 Codex 完成的工作。

每个 Task 开始的内部准备只有一轮：核对依赖 → 把该节决策写入 ADR/精确接口 →
确认实现和测量入口/文件所有权 → 小样本验证 → 实施。该准备属于已批准 Task，
不是新审批。若需改变以上推荐语义才向用户说明增量。阶段结果不达标先在范围内修复；
不能修复且需要新产品决策时，才报告原因、影响和一个明确建议。
