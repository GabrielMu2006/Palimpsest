# CHRON-030 Handoff — 开始 Simulation Worker

Date: 2026-08-31. 用户明确要求“写handoff新开窗口，调用子代理开始下一项任务”。
这批准 **CHRON-030 的实施、必要 ADR、测试与测量，并调用 Luna 子代理**；
不是重新批准已关闭修复，也不授权自动进入031–036、Phase2、commit/push/PR或远端设置。

## 当前状态 / Dependencies

- 原工作区：`/Users/gabrielmu/Documents/Palimpsest`；分支 `phase-1-planning`；
  HEAD `e5b0aeb676372a123dd8c27190e94b6a606d498c`。
- **核心实现大量未提交/未跟踪，旧HEAD不是交接成果。** 新任务承接当前工作树，
  不得从干净main/旧HEAD重做。先核对迁移后的文件及下面74个源码哈希；缺失时修复
  交接，不通过重置、清理或伪造依赖完成来继续。不得把历史修复变更当作自己新增。
- CHRON-027–029 + P1-KERNEL-REPAIR-V2 已本地验证。debug/release各330项执行，
  doctest、Clippy/MSRV/Godot smoke通过；40正式+1smoke时间样本、15冷进程验证通过。
- 基线证据：[V2报告](../reports/P1_KERNEL_REPAIR_V2.md)、
  [74源码/4二进制身份](../reports/data/kfix-v2-environment.json)、
  [原始数据复算](../reports/data/kfix-v2-validation.json)。新worktree不要求复制target二进制。
- Master SHA256仍为 `a6fa0654582eca360b3fc8be6d7989200d310707677f841e58130c301b2de5ea`。
  完整十年、Phase1 FPS/整机预算、hosted CI仍未验收。

## Reading route

完整读取四份必读规范，然后当前进度、[CHRON-030](../tasks/CHRON-030.md)、
[ADR-0015](../adr/ADR-0015-simulation-worker-command-render-snapshot.md)、
[ADR-0025](../adr/ADR-0025-kernel-repair-completion.md)，及直接相关ADR0022–24、
[执行契约](../EXECUTION_CONTRACT.md)。P1-REMAINING只取030相关D3/配套文件/测量条款；
其旧全局Proposed、旧“尚无kernel”起点不是当前状态，也不授权其他Tasks或远端发布。
读索引/所需章节，不递归加载已关闭修复计划与所有历史报告。本handoff不替代必读规范。

## Scope / API Contract

以CHRON-030现有详细契约为准；以下是必须携带的边界：

- 一个独立的进程内 Simulation 线程独占WorldKernel；安全Rust，标准库，禁止Godot主线程跑tick。
- 有界64命令队列，Full/Closed显式返回；入队序号和最终应用/拒绝ack分离。
- Pause/Resume、速度1/5/20/100/1000/MAX、Step、AdvanceTo、Shutdown。
  初始暂停；Step每步1模拟秒，0无副作用，最多1000，运行时拒绝；AdvanceTo仅暂停可用。
- 速度只控制墙钟pacing，不改cadence/权重/模拟内容；MAX不等待墙钟。
- 完整due边界应用命令/发布；yield不能误报目标完成。队列满时仍有独立停止路径。
- 最新完整不可变快照、单调发布序号，exchange最多拥有两份；读者只保留当前帧。
  10Hz墙钟发布目标，暂停/step强制刷新；不承诺零延迟或强制收回外部Arc。
- 现有Kernel的sites/next_due及人物/trace读取均为Result；Faulted不再推进或构造新DTO，
  必须暴露故障并保留最后完整发布。不得绕开ADR0025以获取半提交状态。
- 主代理先补齐ADR0015的具体线程/ack保留上界/关闭/错误/pacing契约；普通内部细化
  不另设产品审批。若需改变既定语义，用新ADR记录并按执行契约判断是否真实越界。

## Out of Scope

031 Godot UI、032十年、033规模验收、IPC、并行ECS、async runtime、存储/历史、
新玩法、修改Master/预算/AI权重/native RSS unsafe、全局skills/provider配置。
不使用OpenCode/opencode-go，不提交/推送/改远端，不把“下一项”扩大为剩余整阶段。

## Files Modified / Allowed and ownership

本交接只新增本文，更新CURRENT_PROGRESS、TASK_INDEX、CHRON-030状态；没有实现worker。
新任务按030规范的已有范围细化文件清单，优先Core-only，不提前接入031客户端。

- 主代理：`crates/sim-core/src/worker.rs`及必要同模块结构、`src/lib.rs`导出、
  对应ADR、共享manifest、集成、索引/报告。只有必要的thin bridge/runner接线才触及外层。
- Luna叶子候选：已冻结类型/pacing helpers及自己的单元测试；固定API后的
  `crates/sim-core/tests/worker*.rs`；接口稳定后才分发worker benchmark/adapter。
- 必要配套：`crates/sim-core/examples/worker_bench.rs`及私有支持；
  `tools/bench-memory/{src/main.rs,tests/cli.rs,README.md}`新case；已有依赖接线；
  `docs/reports/CHRON-030_SIMULATION_WORKER.md`、`data/chron-030-*`及相关架构/性能/状态。
- 写入文件不得重叠，lib/manifest/ADR由主代理持有；不得同时全项目格式化或争用构建。
  旧V2数据及其scope/source manifest是历史快照，不随新任务重新写hash来假装未变化。

## Agent dispatch / readiness

使用 `/Users/gabrielmu/.codex/skills/codex-luna-dispatch/SKILL.md` 与任务包模板。
请求原生 `gpt-5.6-luna`、medium、`fork_turns=none`；不递归、不用侧栏任务模拟子代理。

| 子任务 | 当前判断 | 新主代理的启动步骤 |
|---|---|---|
| 生命周期/并发/ack/public API | KEEP_LOCAL | 亲自确定契约和失败路径，先写ADR再实现 |
| 速度类型/纯pacing helpers + unit tests | PREPARE→READY | 根据D3冻结准确签名、溢出/舍入规则及文件所有权后派Luna |
| worker命令/发布回归 | PREPARE→READY | API冻结后独立文件派Luna；不能让它猜并发契约 |
| throughput/RSS adapter | PREPARE→READY | worker稳定并通过关键回归后分发，主代理持有测量协议 |

立即开始主代理准备，在第一个叶子契约READY时实际调用至少一个Luna；不要只写分发计划。
首次spawn后简短公布实际agent/task ID，供旧任务确认已启动；然后自主完成030，不等待旧任务。
最多一次有界返工，仍不合格由主代理接管。只做一次有针对性的父代理审查与必要自动门禁，
不添加重复产品审批或重复重读历史；测试/性能失败仍须真正修复。

## Tests / Benchmark / Definition of Done

- 测试按030覆盖Full/backpressure、ack拒绝、暂停/六速度/Step边界、回退/溢出、
  真实Kernel故障、慢读者/最新完整/不倒退发布、队列满关闭与重复生命周期。
  用受控同步/时钟测试，不靠脆弱sleep断言；墙钟pacing不当作可复现输入轨迹。
- 执行030及P1-REMAINING规定的fmt/Clippy、debug/release workspace、docs、
  exact normal依赖审查、Godot/native必要smoke；主代理独立执行关键回归。
- 创建worker_bench：100人、86400秒同seed/config/终点的direct/worker对照，
  release、2预热+10正式、完整raw/上中位数/min/max。六种pacing用短真实墙钟窗口；
  **不按1×等待一天，不重跑无关的V2全年测量，不把fake clock当性能数据。**
- 测commands/s、sim-s/wall-s、提交到发布延迟、最大队列深度及native RSS；新增用例
  各3冷进程，保留proof/失败/对象存活；测量与构建并发互斥，不把缺仪器改成N/A。
- Done：030契约逐条有代码/测试/测量证据，父代理独立验证、报告实际文件/命令/限制，
  更新当前进度和索引后停止；不自动启动031。现有通过结果只是基线，不代替新验收。

交接本身 Benchmark=N/A（仅文档/任务转移）；检查链接、git diff --check与Master/hash。
