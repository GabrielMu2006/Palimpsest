<!-- Authored by Kimi Code (AI coding agent); content synthesized from Intro/ and docs/. -->

# Palimpsest

> **A world remembers imperfectly.**
> 一个会经历、记住、遗忘，并不断重写自身历史的世界。

**Palimpsest** 是一款以自主世界演化、个体生命与历史书写为核心的沙盒模拟游戏（开发中，早期阶段）。

你不是统治一切的国王，而是一个世界的观察者。世界里的人自主出生、成长、工作、相爱、争吵、迁徙、参战、衰老、死亡；文明自行诞生、扩张、分裂与衰亡。历史不是预先安排好的章节——一场战争必须能真实回溯到三十年前的旱灾、一位领主错误的决定，或两个家族早已被遗忘的争端。

它模拟的不只是"发生了什么"：

- **三层真相分离**——真实发生的历史、人们记住的历史、史官写下的历史，可以互相矛盾。你可以点着史书上的一句话追问"这句话来自哪里"，一路追溯到某份错误的目击证词、一封匿名信，或作者本人对王室长久的不信任。
- **干预只改原因**——你可以降下旱灾、暴雨、疾病或异象，但不能直接把忠诚改成一百、把仇恨从心里删除。你创造原因，世界承担后果。
- **名字的含义**——Palimpsest 是一种被反复擦除、覆盖、重写的古老手稿：新文字写在旧文字之上，旧日的痕迹从未真正消失。

完整愿景见 [Intro/Palimpsest_Full_Introduction.md](Intro/Palimpsest_Full_Introduction.md)（[简版介绍](Intro/Palimpsest_Short_Introduction.md)）。

## 项目状态

| 阶段 | 状态 |
|---|---|
| Phase 0 — 架构验证 | ✅ 2026-08-29 完成并经产品负责人确认（[验证报告](docs/reports/ARCHITECTURE_SPIKE_V1.md)） |
| Phase 1 — 微型世界内核 | 🔨 进行中：19 个任务已规划（[计划](docs/PHASE_1_PLAN.md)），CHRON-018（workspace 边界）已完成，其余待逐任务批准 |
| Phase 2+ — 生命/经济/记忆/战争/史官等 | 📋 仅路线图，见 `MASTER_SPEC.md` |

请注意：仓库目前还没有任何可玩的游戏内容。Phase 0 交付的是经过实测的架构地基——确定性时间、持久实体身份、结构化事件、调度队列、SQLite 事件库、快照、Godot 桥——而不是游戏系统。

## 架构一句话

Rust 模拟核心是权威，完全脱离 Godot 无头运行；Godot 4 只负责窗口、输入与渲染，通过 GDExtension 窄桥获取不可变的渲染快照。详见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。

## 仓库结构

- `crates/` — Rust 模拟核心库（`sim-time` / `sim-entity` / `sim-events` / `sim-scheduler` / `sim-storage` / `sim-core` / `sim-world` / `sim-ai` / `godot-bridge`）
- `apps/headless-runner/` — 无头命令行运行器与基准程序
- `apps/macos-godot/` — Godot 4.7 macOS 客户端工程
- `docs/` — 架构文档、阶段计划、ADR（架构决策记录）、任务规格、验证报告
- `tools/` — 本地 CI 门禁脚本
- `MASTER_SPEC.md` — 最高权威产品规格（只读，CI 用 SHA-256 守护）

## 构建与验证

要求：Rust 1.95+（见 `rust-toolchain.toml`）；客户端另需 Godot 4.7。

```sh
# 完整 Rust 门禁：MASTER_SPEC 哈希、格式、clippy、全部测试、MSRV、基准冒烟
sh tools/ci-rust.sh

# Godot 客户端冒烟（需本机安装 Godot 4.7）
sh tools/ci-godot.sh
```

## 治理方式

- 仓库持续保持公开（产品负责人于 2026-08-30 确认，替代此前私有要求）；`main` 必须保留严格 Rust/Godot 必需检查、管理员保护，禁止强推和删除。远端保护状态必须实际核验，不能只凭文档宣称已生效。
- `MASTER_SPEC.md` 为只读最高权威；与之冲突的需求必须先在 `docs/proposals/` 写变更提案。
- 跨模块公共契约必须先记录 ADR（`docs/adr/`）。
- Phase 1 的每个任务需产品负责人单独批准后才实现；任务规格在 `docs/tasks/`。
- 不删除、不削弱测试；不擅自放宽性能预算。
