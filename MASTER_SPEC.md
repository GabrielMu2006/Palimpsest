# Palimpsest— Master Spec V0

> 临时工程代号：**Palimpsest**  
> 产品类型：自主世界 / 文明 / 个体历史模拟 Sandbox  
> 首发平台：macOS  
> 后续平台：Web  
> MVP 语言：中文  
> 开发模式：Agent-first  
> 基准开发设备：Apple Silicon M5 / 16GB Unified Memory  
> 核心原则：Simulation First / History First / LLM Optional

---

# 1. 产品定义

Palimpsest 不是一款以基地建设或直接控制角色为核心的 RimWorld 类游戏，也不是传统的策略游戏。

它的核心是：

> **创建一个世界，让这个世界真实、自主地生活，然后观察、研究和干预它的历史。**

玩家的主要身份是观察者。

玩家可以：

- 创建世界；
- 让世界从创世时代开始演化；
- 任意控制时间速度；
- 观察文明、聚落、生物自主活动；
- 查看世界级重大事件；
- 查看一个普通角色完整的一生；
- 查看角色的家庭、记忆、信仰、关系、知识和经历；
- 切换到角色的“认知视角”；
- 阅读史官、书信、法令等世界内部文献；
- 研究一段历史究竟依据哪些史料；
- 回到过去查看历史世界状态；
- 对某个人、家族、城市或文明持续关注；
- 通过“上帝事件”影响世界；
- 后续通过自然语言修改世界规则。

游戏不存在胜利条件、失败条件或主线任务。

游戏乐趣来自：

**创造 → 观察 → 发现 → 追踪 → 理解 → 干预 → 继续观察。**

---

# 2. 六条不可破坏的产品原则

## 2.1 世界先存在，故事后产生

不得先随机生成故事文本，再假装这些故事发生过。

必须是：

Simulation State  
→ Entity Actions  
→ Events  
→ Consequences  
→ History  
→ Narrative

例如：

“北境饥荒导致战争”

不能是系统随机抽中的剧情。

必须真实存在类似：

降雨减少  
→ 粮食产量下降  
→ 粮价上涨  
→ 人口迁徙  
→ 边境土地压力  
→ 双方关系恶化  
→ 冲突  
→ 动员  
→ 战争

---

## 2.2 LLM 永远不是 Simulation Truth 的来源

即使关闭：

- 本地模型；
- 网络；
- API；

整个世界仍然必须正常运行。

LLM只能用于：

- 高价值认知增强；
- 自然语言规则解析；
- 可选文本润色；
- 少量复杂人物反思；
- 高级叙事表现。

LLM不能直接决定：

- 人是否死亡；
- 战斗结果；
- 资源数量；
- 国家领土；
- NPC 当前真实位置；
- 世界真实历史。

---

## 2.3 世界真实历史与人物认知必须分离

必须存在至少三层信息：

Simulation Truth  
↓  
Knowledge / Belief  
↓  
Historiography

例如：

真实事实：

王后毒杀国王。

守卫认知：

怀疑王后。

普通市民：

认为国王病死。

官方史书：

声称国王病死。

后世史官：

认为官方说法存在疑点。

三者不得混淆。

---

## 2.4 每个重要结果必须尽量存在因果来源

人物参战不能只是随机：

`join_war = random()`

而应该能够得到：

保护家庭 +31  
忠诚 +24  
朋友参军 +17  
恐惧 -21  
妻子反对 -14  
身体伤势 -8

最终 Utility Score = 68。

玩家版 Why Inspector 可以只显示主要因素。

Developer Mode 则显示完整权重和输入。

---

## 2.5 个体深度优先于人口数字

最终智慧个体暂定：

**≤ 10,000。**

这里的智慧个体是真正拥有：

- 身份；
- 年龄；
-身体；
- 性格；
- 价值观；
- 技能；
- 职业；
- 家庭；
- 多维关系；
- 记忆；
- 知识；
- 信仰；
- 目标；
-经历；

的 Entity。

不允许为了宣传“十万人世界”把大多数角色变成虚假的统计数字。

普通植物、昆虫、鱼类以及远方野生动物可以使用 Population Simulation。

---

## 2.6 模拟深度必须依靠 Simulation LOD 扩展

不允许 10,000 NPC 全部每秒执行完整 AI。

必须实施分级模拟。

---

# 3. 最终世界愿景

最终世界支持完整世界、多文明以及长时间历史。

玩家可以选择：

- 创世时代；
- 部落时代；
- 王国时代；
- 已发展世界；

作为模拟起点。

世界可演化数百至数千年。

主要系统最终包括：

世界地理  
生态  
天气  
资源  
人口  
家庭  
遗传与种族适应  
身体与疾病  
技能与职业  
经济  
生产链  
建筑  
聚落  
组织  
政治  
战争  
犯罪  
司法  
科技  
文化  
语言谱系  
宗教  
神明  
魔法规律  
知识传播  
记忆  
史学  
文献  
历史考据  
世界规则

其中地下系统暂时只作为：

- 矿物；
- 地下水；
- 洞穴资源；

不开发完整地下文明。

---

# 4. MVP 的唯一核心问题

MVP 不需要证明：

“我们能实现多少功能。”

MVP 必须证明：

> **一个足够小的世界在无人控制的情况下运行 200 年以后，会不会形成值得玩家主动研究的历史。**

MVP 基准：

智慧人口：100～200  
聚落：2～5  
智慧种族：2～3  
Local Map：128×128  
生态物种：约5～15  
历史长度：≥200年  
首发语言：中文  
平台：macOS

---

# 5. MVP 必须实现的闭环

MVP 必须具备：

世界生成  
→ 地形 / 基础生态  
→ 智慧 NPC  
→ Needs  
→ Personality  
→ Values  
→ Utility AI  
→ Skills  
→ Profession  
→ Family  
→ Birth / Aging / Death  
→ 多维关系  
→ Memory  
→ Knowledge Bias  
→ 基础工作  
→ 基础生产链  
→ 聚落  
→ Tile Construction  
→ 生态  
→ 个体战斗  
→ 简单政治冲突  
→ 简单战争  
→ Event Store  
→ Headless Historical Simulation  
→ World View  
→ Archive View  
→ Entity View  
→ History Replay  
→ Historian  
→ 中文 NLG  
→ Historical Sources  
→ Watch System  
→ Significance System  
→ 5～10 个 God Actions

这是 MVP 的完整边界。

---

# 6. MVP 明确不做

以下系统不得因为 Agent “觉得顺便做一下”而进入 MVP：

复杂宗教  
真实神明 Agent  
程序化魔法发现  
完整科技演化  
语言历史演化  
复杂犯罪司法  
完整疾病生态  
复杂市场经济  
高度涌现政治制度  
LLM NPC Agent  
自然语言 Rule Editor  
Lua/Rhai Mod  
大型城市  
复杂地下世界  
万人正式游戏世界  
Web Client  
多人模式  
完整商业发布系统

这些功能不是取消。

只是延期。

---

# 7. 推荐技术架构

## 7.1 Simulation Core

语言：

**Rust**

运行层：

**独立 Rust Library**

推荐使用：

`bevy_ecs`

作为 Runtime ECS。

但是：

`bevy_ecs::Entity`

绝不能成为永久 Entity ID。

必须额外拥有：

`EntityId(u64)`

作为持久化身份。

例如：

Person #81271

在：

事件  
关系  
史书  
存档  
数据库  
组织

中永远通过 `EntityId` 引用。

ECS Entity 只作为 Runtime Handle。

---

# 8. 客户端架构

MVP：

**Godot 4 macOS Client**

Godot负责：

- Window；
- Input；
- UI；
- Tile Rendering；
- Camera；
- Animation；
- Panels；
- Inspector；
- Archive Browser。

Rust负责：

- Simulation；
- AI；
- History；
- Entities；
- Event Store；
- NLG；
- Rules；
- Persistence。

通过：

Godot GDExtension / godot-rust

连接。

不得让 Godot Scene Tree 成为世界真实数据源。

正确方向：

Godot：

“Person 8127 现在应该显示在哪里？”

Rust：

返回 Render Snapshot。

而不是 Godot Node 决定 Person 8127 的世界状态。

---

# 9. Web 架构

Web 不进入 MVP。

长期：

Rust Core  
↓  
WASM  
↓  
Web Client

Web 客户端不要求复用 Godot 客户端。

核心只要求：

Simulation Core 的 Domain API 与 UI 无关。

---

# 10. 时间系统

世界内部时间统一使用：

`SimInstant`

建议底层：

**整数秒**

例如：

`SimInstant(i64)`

但绝对不意味着所有系统每秒更新。

不同系统拥有自己的 Scheduler。

例如：

Combat：

1～5秒

观察中的人物行为：

5秒～数分钟

普通人物：

10分钟～1小时

遥远人物：

数小时

生态：

数小时～天

政治：

天～月

科技：

月～年

地理：

月～年

因此游戏不采用：

“每秒遍历所有 Entity”。

而采用：

**Scheduled / Event-driven Simulation。**

---

# 11. Simulation LOD

至少建立四级。

## LOD 0 — Active

对象：

当前 Local Map  
战斗  
玩家关注角色  
重大历史人物  
关键事件

特点：

最高精度。

---

## LOD 1 — Detailed

对象：

当前 Region  
附近聚落  
重要人物

特点：

减少路径和行为更新频率。

---

## LOD 2 — Coarse

对象：

远方普通智慧个体。

仍然保存：

身份  
家庭  
关系  
职业  
目标  
记忆摘要  
位置

但行为以：

数小时 / 天

为步长。

---

## LOD 3 — Aggregate

主要用于：

野生动物种群  
植物群落  
资源  
远方生态

而不是把智慧 NPC 彻底变成统计数据。

---

# 12. 玩家关注系统

任何 NPC 都可：

⭐ Watch

被 Watch 后：

Simulation Importance ↑  
History Retention ↑  
Event Notification ↑  
LOD Priority ↑

这意味着：

一个6岁的普通铁匠之子可以因为玩家关注而被完整观察到81岁死亡。

关注不能改变 NPC 的决策。

只能改变：

**我们观察他的精度。**

---

# 13. NPC 核心模型

每个智慧 NPC 至少包含以下 Domain。

Identity  
Body  
Needs  
Personality  
Values  
Preferences  
Skills  
Knowledge  
Memory  
Goals  
Relations  
Family  
Profession  
Organization Membership  
Beliefs  
Inventory Summary  
Location  
Current Action

---

# 14. Utility AI

MVP 不采用实时 LLM Agent。

基本流程：

Perception  
↓  
Needs  
↓  
Available Actions  
↓  
Utility Calculation  
↓  
Action Selection  
↓  
Execution  
↓  
Event  
↓  
Memory

评分必须可解释。

禁止出现无法解释的：

`random_action()`。

随机性可以存在，但属于评分扰动，而不是行为系统主体。

---

# 15. Goal 系统

短期：

Eat  
Sleep  
Work  
Travel  
Socialize  
Protect

长期：

Raise Family  
Become Wealthy  
Gain Status  
Seek Revenge  
Serve Religion  
Become Scholar

MVP 只实现有限长期 Goal。

后续逐渐加入真正 GOAP / Planning。

---

# 16. Personality 与 Values

Personality 与 Values 必须分开。

例如：

Personality：

Bravery  
Patience  
Greed  
Empathy  
Discipline  
Curiosity  
Sociability

Values：

Family  
Wealth  
Honor  
Religion  
Freedom  
Tradition  
Knowledge  
Power

经历可逐渐改变两者。

但改变应缓慢。

---

# 17. 多维人际关系

禁止单一：

`relation = 78`

推荐：

Affection  
Trust  
Respect  
Fear  
Attraction  
Resentment  
Loyalty  
Familiarity

同时维护：

RelationshipHistory。

于是：

“爱但不信任”

必须成为合法状态。

---

# 18. Memory System

Memory 至少拥有：

event_id  
subject  
timestamp  
salience  
emotion  
confidence  
decay  
source  
distortion

记忆允许：

遗忘  
强化  
错误归因  
扭曲

MVP 不实现普通情况下人为植入虚假记忆。

未来魔法等系统可以使用。

---

# 19. Knowledge Architecture

这是整个游戏最关键的数据设计之一。

必须严格区分：

Fact  
Observation  
Memory  
Belief  
Claim  
Rumor  
Document

例如：

Fact：

王后杀死国王。

Observation：

守卫看见王后的人进入房间。

Belief：

守卫认为王后可能杀死国王。

Claim：

守卫告诉朋友“王后可能杀死国王”。

Rumor：

“王后杀死国王。”

Document：

史官引用这个传闻。

不得使用一张万能 `knowledge` 表把这些概念混在一起。

---

# 20. 信息传播

MVP 使用混合方案。

重大信息：

必须拥有传播来源。

例如：

Witness  
→ Merchant  
→ Tavern  
→ Traveler  
→ Town

普通闲聊：

可以 Region 级聚合。

这样保证：

重要历史能够追溯来源，

同时不会追踪世界里的每一句废话。

---

# 21. Family System

最终必须支持：

出生  
父母  
兄弟姐妹  
伴侣  
婚姻  
子女  
分离  
继承  
家族

文化可以影响：

婚姻制度  
成年年龄  
继承制度  
家族权力

MVP 先采用少量 Culture Rule。

---

# 22. Skill 与 Profession

Profession 不是硬编码身份。

例如：

Blacksmith

实际上来自：

Smithing Skill  
Relevant Knowledge  
Workshop Access  
Guild Status  
Actual Work History  
Social Recognition

因此一个角色可以同时拥有：

Farmer  
Militia  
Poet

等不同社会角色。

技能通过：

实践  
训练  
教学  
经历

增长。

---

# 23. 身体与疾病

采用：

**简化部位级身体模型。**

MVP 建议：

Head  
Torso  
Left Arm  
Right Arm  
Left Leg  
Right Leg

支持：

Health  
Pain  
Bleeding  
Fracture  
Infection  
Disability

疾病支持：

传播方式  
潜伏期  
严重程度  
免疫

但禁止进入真实医学模拟。

---

# 24. Combat

目标：

DF 风格的个体战斗结果，

但不复制 DF 极端身体组织复杂度。

战斗因素：

身体部位  
武器  
护甲  
技能  
体力  
士气  
恐惧  
疼痛  
出血  
伤势

大型战争：

必须使用分级模拟。

局部关键战斗：

个体精算。

大规模战场：

Formation / Unit Resolution。

关键 NPC：

即使在大规模战斗中也拥有较高模拟精度。

---

# 25. Economy

MVP 不实现开放市场经济。

采用真实生产链：

Resource  
→ Production  
→ Storage  
→ Consumption

例如：

Wood  
Wheat  
Food  
Iron Ore  
Iron  
Tools  
Weapons  
Clothes

生产需要：

资源  
劳动  
技能  
时间  
设施

最终扩展：

价格  
贸易  
商人  
税收  
财富  
阶层。

---

# 26. 物品系统

按照用户选择：

绝大多数物品聚合。

例如：

Iron × 52  
Wheat × 238

只有真正重要物品升级为 Entity：

Artifact  
WrittenWork  
Royal Regalia  
Relic

因此不得为每粒小麦创建 Entity。

---

# 27. Ecology

生态必须真实参与历史因果。

采用：

Population + Local Entity Hybrid。

远方：

Wolf Population = 84

Local观察：

实例化附近狼。

允许：

食物网  
繁殖  
捕食  
疾病  
迁徙  
季节  
栖息地  
灭绝

生态变化可以导致：

粮食减少  
野兽袭击  
迁徙  
经济问题  
战争。

---

# 28. Geography

最终支持：

河流变化  
森林变化  
土地利用  
湿地  
荒漠化  
洪灾  
土壤退化

但采用：

低频演化模型，

不得实现连续地质/流体模拟。

---

# 29. World / Region / Local

地图全部采用方形 Cell。

最终结构：

WORLD GRID  
↓  
REGION  
↓  
LOCAL CHUNK  
↓  
TILE

MVP：

Local = 128×128。

长期：

Village = 128×128  
Town = 多 Chunk  
City = 多 Local Chunk

理论上世界任何位置均可展开为 Local。

未观察地区：

通过 Region Simulation。

观察以后：

生成 Local。

离开以后：

降低 LOD。

---

# 30. Settlement Construction

当前 Local 中真实模拟：

Planning  
→ Resource Gathering  
→ Transport  
→ Construction  
→ Completion

建筑拥有：

material  
builder  
quality  
condition  
purpose

关键建筑拥有 History。

普通建筑远程使用 LOD。

---

# 31. Organization

组织是独立 Entity。

例如：

Family  
Army  
Church  
Guild  
School  
Merchant Association  
Secret Society  
Bandit Group

至少包含：

members  
leaders  
resources  
goals  
relations  
history

组织可以：

成立  
分裂  
合并  
改名  
灭亡。

---

# 32. Politics

MVP 只实现：

Settlement Leadership  
Faction Relation  
Resource Conflict  
War

后续才加入：

真正涌现的权力体系。

最终政治权力来源可能包括：

Military  
Wealth  
Religion  
Land  
Bloodline  
Popular Support  
Institution。

---

# 33. Historical Significance

每个历史事件拥有：

`SignificanceScore`

影响因素包括：

人物重要度  
伤亡  
地理范围  
政治影响  
经济影响  
文化影响  
稀有程度  
持续时间

用于：

Event Feed  
Watch  
Auto Pause  
Archive Ranking  
History Retention。

---

# 34. Event Store

这是 Core 中与 Simulation 同等级的模块。

禁止简单用：

`Vec<String>`。

事件必须结构化。

建议：

EventId  
Timestamp  
EventType  
Actors  
Targets  
Location  
Causes  
Consequences  
Visibility  
Significance  
Metadata

---

# 35. History Persistence

采用：

SQLite

推荐：

WAL Mode

+

周期 Snapshot。

数据库包含：

events  
entities metadata  
relations  
claims  
documents  
written works  
snapshot indexes  
watch state  
world config

大块 Snapshot：

二进制序列化  
+ zstd 压缩。

---

# 36. 为什么不是“保存每一次移动”

采用三种数据寿命：

HOT

最近高精度历史。

WARM

重要结构事件。

COLD

远古历史。

例如：

“某人从床走到门口”

不永久保存。

而：

迁徙  
结婚  
战争  
负伤  
职业改变  
重要关系变化

永久保存。

---

# 37. Full Archive / Compact History

提供两种模式。

Full Archive：

保留更多微观状态与 Snapshot。

Compact：

更早压缩低价值历史。

但：

World-internal History 是否失传

与：

Simulation Truth 是否保留

是两个独立概念。

---

# 38. Historical Replay

History Replay 不要求：

“从 Seed 重新模拟到那个时间点。”

因为世界不是完全确定性的。

正确实现：

Historical Snapshot  
+ Stored State Delta

恢复历史状态。

因此 Replay 是：

**查看已经发生过的历史。**

不是：

重新计算过去。

---

# 39. World File

导出：

`.world`

建议内部：

manifest.json  
world.db  
snapshots/  
content-manifest.json  
preview.png

导出流程：

Pause Simulation  
→ SQLite Checkpoint  
→ Validate DB  
→ Create Manifest  
→ Compress Package

禁止直接在 WAL 写入过程中粗暴复制数据库文件。

---

# 40. World Archive

这是与地图同等级的主要玩法。

三大核心界面：

WORLD

当前世界。

ARCHIVE

过去世界。

ENTITY

某个对象。

三者必须大量 Cross-link。

---

# 41. Archive 支持的对象

Person  
Family  
Settlement  
Civilization  
Organization  
War  
Battle  
Religion  
Written Work  
Artifact  
Species  
Disaster  
Place

---

# 42. World-internal / Omniscient

必须支持两个视角。

World-internal：

只展示世界当前真正留下来的知识。

Omniscient：

展示 Simulation Truth。

例如一件谋杀案所有文献都被毁：

World-internal：

“没有可靠记录。”

Omniscient：

仍然看到真实事件。

---

# 43. Historian

史官是游戏核心职业之一。

MVP 至少包含：

Court Historian  
State Historian  
Religious Historian  
Independent Historian  
Traveling Historian

史官拥有：

Knowledge  
Sources  
Bias  
Patron  
Personality  
Writing Skill  
Historical Method

---

# 44. Historiography

必须区分：

Mistake  
Bias  
Propaganda  
Forgery

史官可以：

误解事实；

选择性引用；

主动宣传；

故意伪造；

删除资料。

但系统内部必须知道：

Claim 来源是什么。

---

# 45. WrittenWork

每一部重要作品是真实 Entity。

例如：

WrittenWork #812

包含：

title  
author  
created_at  
type  
subject  
sources  
claims  
edition  
text

生成后：

自然语言正文永久保存。

不得每次点击重新生成。

---

# 46. Primary Sources

MVP 至少支持少量：

Letter  
Diary  
Decree  
Treaty  
Trial Record  
Will  
War Declaration

这些文本可以成为后世史官引用的 Source。

---

# 47. 文献生命周期

长期支持：

Original  
Copy  
Edition  
Edited Edition  
Censored Edition  
Fragment  
Lost Work

允许：

禁书  
焚书  
残卷  
抄写错误  
版本差异。

---

# 48. NLG 是核心模块

禁止把 NLG 当成：

`format!("{} killed {}", a, b)`。

正式 Pipeline：

Fact / Claim  
↓  
Content Selection  
↓  
Narrative Planning  
↓  
Text IR  
↓  
Lexicalization  
↓  
Sentence Realization  
↓  
Discourse Realization  
↓  
Written Text

---

# 49. Text IR

语言无关。

例如：

Proposition:

DEATH  
subject = King Arven  
time = Year 421  
certainty = HIGH  
source = OfficialRecord

Renderer 再决定中文：

“阿尔文王于421年去世。”

未来 EnglishRenderer：

“King Arven died in 421.”

---

# 50. 中文 NLG

MVP 只打磨中文。

需要建立：

Lexicon  
Sentence Templates  
Connectives  
Time Expressions  
Title Generator  
Honorific Rules  
Name Renderer  
Pronoun Resolver  
Paragraph Planner

禁止为了多语言而同时降低中文质量。

---

# 51. NLG Style

最终文本风格由：

Author  
× Culture  
× Era  
× Scholarly Tradition

决定。

MVP：

Author + Culture。

例如：

skepticism 高：

“据称”  
“尚无证据”  
“这一记载存在争议”

religiosity 高：

“神意”  
“圣兆”

royal loyalty 高：

减少王室负面叙事。

---

# 52. 文本可解释性

玩家选中一句：

“王后可能参与了谋杀。”

可以查看：

Source：

Guard Testimony #12  
Anonymous Letter #77

Author Confidence：

31%

Bias：

Anti-Court +18

这是项目非常重要的差异化功能。

---

# 53. LLM Text Enhancer

LLM 不作为默认 NLG。

Pipeline：

Text IR  
↓  
Rule NLG  
↓  
Canonical Draft

可选：

Canonical Draft  
+ Claims  
+ Author Style  
↓  
LLM  
↓  
Enhanced Draft

LLM不得读取 Simulation Truth。

并且增强后的文本不得增加不存在的 Claim。

---

# 54. LLM Architecture

AI分三级。

Level 0：

Simulation / Utility AI

永远存在。

Level 1：

Tiny Local Model

可选。

Level 2：

Remote BYOK API

可选。

任何一级失败：

不得停止 Simulation。

---

# 55. 本地模型

后续测试：

Qwen 0.6B / 0.8B 级  
Gemma 270M 级

用途：

Intent Classification  
Simple Rule Parsing  
Text Categorization  
Tiny Summary

不允许默认承担：

复杂政治思考  
长期人生规划  
复杂心理 Agent。

---

# 56. Local LLM Runtime

Rule Editor 的结构化任务优先测试：

llama.cpp

原因：

可使用 JSON Schema / grammar constrained output。

Apple 专属自然文本后端可以额外支持：

MLX / MLX-LM。

但是必须封装：

`LlmBackend` Trait。

游戏其他系统不得直接依赖某个模型运行库。

---

# 57. Remote LLM

采用：

BYOK。

用户自行配置：

Provider  
Base URL  
Model  
API Key

不得在 MVP 建官方服务器。

支持：

每日 token limit  
每小时 request limit  
消费预算

队列过载：

Remote  
↓  
Local  
↓  
Rule AI

自动降级。

---

# 58. Rule Engine 最终方向

玩家最终不需要写 JSON、Lua 或 Rust。

用户：

“冬天让狼更容易饥饿。”

LLM：

生成 RuleIR。

RuleIR：

Trigger  
Condition  
Target  
Effect  
Duration  
Probability

然后：

Schema Validator  
↓  
Semantic Validator  
↓  
Cost Estimator  
↓  
Conflict Detector  
↓  
Preview  
↓  
Execution

---

# 59. LLM 没有直接代码执行权

任何情况下：

自然语言 → 任意 Rust/Lua → 执行

都不是默认路线。

初期 Rule System 必须是：

**whitelisted structured operations。**

---

# 60. Safe Mode / God Mode

Safe Mode：

所有 Rule Diff 必须确认。

God Mode：

通过 Validator 的规则立即执行。

---

# 61. MVP God Actions

MVP 不实现自然语言 Rule Editor。

只提供约 5～10 个固定事件，例如：

Heavy Rain  
Drought  
Fire  
Disease Outbreak  
Spawn Dangerous Animal  
Resource Discovery  
Omen  
Storm  
Food Blessing  
Migration Trigger

注意：

玩家只能制造事件。

不能：

`NPC.loyalty = 100`

---

# 62. 玩家不能直接修改 NPC 内部状态

例如玩家希望帮助某人：

不能：

Strength = 100。

可以：

赐予神器。

不能：

Faith = 100。

可以：

降下神迹。

NPC 是否因此改变：

由模拟决定。

---

# 63. Seed

Seed 保留。

Seed 决定：

初始地形  
资源  
气候  
物种  
初始人物  
初始文明

同 Seed：

初始世界应相同或高度一致。

但是未来历史：

不保证一致。

---

# 64. 世界生成

Normal：

简单配置。

Advanced：

World Size  
Climate  
Ocean  
Mountain  
Species  
Starting Era  
Magic Strength  
Disaster Rate  
Historical Pre-simulation

MVP 只开放少量参数。

---

# 65. Historical Pre-simulation

必须支持：

Headless Simulation。

500 年预演过程中：

不渲染 NPC 动画。

只显示：

Year  
Population  
Civilizations  
Major Events

并允许玩家：

随时 Stop → Enter World。

---

# 66. Time Controls

至少：

Pause  
1×  
5×  
20×  
100×  
1000×  
MAX

速度越高：

自动提升 Simulation LOD。

MAX：

关闭无用渲染。

高级设置未来可允许手动控制精度。

---

# 67. Important Event Feed

高速模拟必须有：

Significance Threshold  
Watch List  
Auto Pause

例如：

“关注角色死亡”

可以自动暂停。

---

# 68. UI MVP

至少实现：

Main Menu  
World Creation  
World View  
Archive  
Entity Inspector  
Event Feed  
Watch Panel  
Historian / WrittenWork Reader  
Time Control  
God Actions  
Developer Mode

---

# 69. 2D Visual Direction

采用：

RimWorld 式简洁 2D Tile。

重点：

可读性。

人物应能识别：

种族  
职业  
装备  
受伤状态。

MVP 不追求大量动画。

---

# 70. Developer Mode

Phase 0 就开发。

必须实时显示：

Simulation TPS  
Active Entities  
LOD Distribution  
Events/s  
Memory  
Database Size  
Scheduler Queue  
Pathfinding Jobs  
Utility Decisions

---

# 71. Developer Entity Inspector

可以检查：

Entity Components  
Needs  
Personality  
Goals  
Relations  
Memories  
Knowledge  
Current Action

---

# 72. Decision Trace

Developer Mode 必须回答：

“这个 NPC 为什么做这个动作？”

显示完整 Utility Calculation。

玩家普通 Why 功能：

只显示简化原因。

---

# 73. M5 16GB Performance Contract

第一开发硬件：

Apple M5 / 16GB。

所有性能决策优先保证该设备稳定。

MVP 目标：

100～200 NPC  
128×128 Local  
正常 UI 60 FPS  
200 年 Headless Simulation 可稳定完成  
关闭 LLM 可完整游玩

后续 Scale Gates：

1,000  
3,000  
5,000  
10,000

任何一次扩容：

必须 Benchmark。

---

# 74. 内存预算

暂定目标而非永久承诺：

MVP Core + Client：

尽量 < 3GB RSS。

10K Simulation：

尽量 < 5GB Core/Client。

Tiny LLM 开启后：

整套环境尽量 < 7GB。

具体指标必须在 Architecture Spike 后重新报告给产品负责人确认。

Agent 不得自行放宽性能预算。

---

# 75. Performance Test Suite

必须拥有：

bench_100_entities  
bench_1k_entities  
bench_3k_entities  
bench_5k_entities  
bench_10k_entities

以及：

bench_event_store  
bench_snapshot  
bench_relationship  
bench_memory  
bench_utility_ai  
bench_pathfinding  
bench_nlg  
bench_history_query。

---

# 76. Chaos Simulation Test

每个重要 Milestone 必须运行：

Headless World Simulation。

验证：

没有人口瞬间灭绝；  
资源不会无穷增长；  
不存在 NaN；  
不存在无限循环；  
Entity Reference 不悬空；  
数据库一致；  
长时间运行无明显内存泄漏。

---

# 77. Emergence Test

除了普通 Unit Test，还要测试历史质量。

例如自动输出：

Population Curve  
Deaths by Cause  
Family Distribution  
Wars  
Migration  
Resource Shortages  
Top Historical Events

由 Agent 生成：

`simulation_report.md`

供人工验收。

---

# 78. Repository

建议：

Palimpsest/

apps/
    macos-godot/

crates/
    sim-core/
    sim-time/
    sim-entity/
    sim-ai/
    sim-memory/
    sim-relation/
    sim-world/
    sim-ecology/
    sim-economy/
    sim-combat/
    sim-settlement/
    sim-history/
    sim-nlg/
    sim-rules/
    sim-storage/
    sim-debug/
    godot-bridge/

content/
    cultures/
    species/
    resources/
    professions/
    recipes/
    nlg/
    events/

docs/
    MASTER_SPEC.md
    ARCHITECTURE.md
    ENTITY_MODEL.md
    EVENT_MODEL.md
    NLG_SPEC.md
    PERFORMANCE.md
    ROADMAP.md

    adr/
    proposals/

benchmarks/

tests/
    simulation/
    regression/
    worlds/

tools/

---

# 79. 文档权力等级

最高：

MASTER_SPEC.md

Agent 无权修改。

其次：

Architecture Decision Records。

再次：

Module Specs。

最后：

Task Specs。

如果实现与 Master Spec 冲突：

必须停止。

创建：

Change Proposal。

由产品负责人决定。

---

# 80. AGENTS.md

必须明确：

不得擅自扩大 Scope。  
不得修改 MASTER_SPEC。  
不得跨模块重构。  
不得删除测试解决失败。  
不得为了性能删除产品核心需求。  
不得把 LLM 变成必需依赖。  
不得把 Godot State 当 Simulation Truth。  
不得为完成 MVP 删除未来扩展接口。

---

# 81. ADR

任何涉及以下内容必须产生 ADR：

Database Change  
Entity ID Design  
ECS Change  
Serialization Change  
Godot Bridge Change  
AI Architecture Change  
History Retention Change  
NLG Architecture Change  
Rule IR Change

---

# 82. Agent Task 原则

一个 Task：

只拥有一个明确目的。

必须包含：

Context  
Scope  
Out of Scope  
Files Allowed  
API Contract  
Tests  
Benchmark  
DoD

不得给 Agent：

“继续完善游戏。”

这种开放指令。

---

# 83. Phase 0 — Architecture Spike

目标：

证明架构可行。

任务：

P0-01 创建 Rust Workspace。  
P0-02 创建 Godot 4 macOS 工程。  
P0-03 接通 godot-rust。  
P0-04 建立 EntityId。  
P0-05 建立 SimClock。  
P0-06 实现 Scheduler。  
P0-07 创建 10K Dummy NPC Benchmark。  
P0-08 创建 Event Throughput Benchmark。  
P0-09 Godot 渲染 128×128 Tile。  
P0-10 实现 Developer Overlay。  
P0-11 建立 CI。  
P0-12 建立 AGENTS / ADR / Proposal 流程。

DoD：

M5 16GB 上通过。

Godot 能从 Rust 获得 Render Snapshot。

Simulation 可以完全无 Godot Headless 运行。

---

# 84. Phase 1 — Micro World Kernel

实现：

World Grid  
Terrain  
Local Tile  
Person Entity  
Basic Movement  
Time  
Needs  
Basic Utility AI

目标：

100 NPC 可以：

移动  
进食  
睡眠  
工作。

DoD：

连续模拟 10 年无崩溃。

---

# 85. Phase 2 — Life Simulation

实现：

Age  
Birth  
Death  
Family  
Personality  
Values  
Skills  
Profession  
Relations

目标：

第一次真正出现：

完整人生。

DoD：

50 年后存在：

多代家庭。

---

# 86. Phase 3 — Settlement / Economy / Ecology

实现：

Basic Resource  
Recipe  
Production  
Storage  
Building  
Construction  
Plants  
Wildlife Population  
Season

目标：

聚落能够依靠资源长期生存。

允许：

资源危机。

---

# 87. Phase 4 — Memory / Knowledge

实现：

Observation  
Memory  
Belief  
Rumor  
Information Transfer

目标：

同一事件产生不同 NPC 认知。

这是第一次验证：

Simulation Truth ≠ Personal History。

---

# 88. Phase 5 — Combat / Conflict

实现：

Body Parts  
Weapon  
Injury  
Combat  
Faction Relation  
Simple War

目标：

战争必须从世界状态产生。

不能用固定“每20年随机战争”。

---

# 89. Phase 6 — History Infrastructure

实现：

Structured Event  
Significance  
SQLite  
Snapshot  
Historical Replay  
Watch

目标：

能够：

地图 → 事件 → 人物 → 人生 → 过去。

---

# 90. Phase 7 — Historian & NLG

这是 MVP 最重要阶段之一。

实现：

Historian  
Source Gathering  
Claim  
Narrative Planner  
Text IR  
Chinese NLG  
WrittenWork  
Primary Sources  
Source Inspector

目标：

一个史官可以依据自己的认知：

写出真实永久保存的历史作品。

不同史官：

对同一事件产生不同文本。

---

# 91. Phase 8 — God Actions & MVP UI

实现：

God Event  
World View  
Archive  
Entity  
WrittenWork Reader  
Event Feed  
Auto Pause  
Watch  
Simplified Why

完成产品闭环。

---

# 92. Phase 9 — 200-Year MVP Validation

创建标准测试：

Seed A  
Seed B  
Seed C  
Seed D  
Seed E

每个：

100～200 NPC  
模拟 ≥200年。

必须至少人工检查：

一条多代家庭史；  
一条战争因果；  
一个信息偏差案例；  
一个关系变化案例；  
一个史官误记案例；  
一个史料失传案例；  
一部完整史书；  
一次 God Action 后续影响。

通过后：

MVP 才算完成。

---

# 93. Alpha 1 — Scale

人口：

500 → 1000。

重点：

Performance  
LOD  
Storage。

不新增大量系统。

---

# 94. Alpha 2 — Advanced Society

开始加入：

Organization 深化  
Emergent Politics  
Trade  
Inheritance  
Law  
Important Crime Investigation。

---

# 95. Alpha 3 — Knowledge Civilization

加入：

Technology Discovery  
Knowledge Transmission  
Lost Technology  
Scholar System  
Libraries  
Academic Tradition。

---

# 96. Alpha 4 — Religion

加入：

Religion Generation  
Religious Organization  
Doctrine  
Schism  
Religious Conflict  
Myth。

此阶段仍不要求神真的存在。

---

# 97. Alpha 5 — True Gods & Magic

加入：

Actual Gods  
God Agent  
Supernatural Entity  
Hidden Magical Laws  
Magical Discovery。

核心：

世界真实魔法规律

≠

文明理解的魔法理论。

---

# 98. Alpha 6 — Language History

只实现：

Language Family  
Naming Convention  
Personal Names  
Place Names  
Historical Sound Changes。

不开发完整自然语言。

---

# 99. Alpha 7 — Natural Language Rule Editor

加入：

User Prompt  
↓  
LLM Parser  
↓  
RuleIR  
↓  
Validator  
↓  
Cost Estimate  
↓  
Preview  
↓  
Apply

第一版 Rule IR 严格白名单。

禁止任意代码。

---

# 100. Alpha 8 — Local LLM

评测：

270M  
600M  
800M  
1B 级模型。

建立专门测试集：

SimpleRule-100  
ComplexRule-100  
AmbiguousRule-100  
InvalidRule-100  
ChineseRule-100

评价：

Schema Validity  
Semantic Accuracy  
Unsafe Rule Rejection  
Latency  
RAM  
Energy

只有通过标准：

才成为正式 Local Backend。

---

# 101. Alpha 9 — Scale to 10K

依次：

1000  
3000  
5000  
10000。

不允许直接从1000跳到10000。

每一级生成：

Performance Report。

性能优化不能牺牲：

Memory  
History  
Identity  
Causality。

---

# 102. Web

只有当：

Simulation Core API

稳定以后才开始。

Rust：

WASM。

Web 可以降低：

Local Simulation Scale  
History Cache  
Local LLM。

但不得建立第二套 Simulation Logic。

---

# 103. 开源策略

当前不决定许可证。

代码结构保证：

Core 可独立发布。

Client / Content / Assets 可独立闭源。

MVP/Alpha：

不承诺任何开源范围。

以后再决定。

---

# 104. Save Compatibility

Alpha：

不保证。

因此允许架构重构。

但是所有存档都要有：

schema_version  
simulation_version  
content_version。

Beta 后再设计 Migration Policy。

---

# 105. 单机原则

永久：

Single-player local simulation。

不开发：

Network Sync  
Authoritative Server  
Multiplayer。

世界可以：

Export / Import。

---

# 106. Master Spec Governance

Agent 不得修改此文件。

发现问题：

创建：

`docs/proposals/CP-XXXX.md`

内容：

Problem  
Current Spec  
Proposed Change  
Reason  
Impact  
Migration  
Alternative

只有产品负责人确认后：

才能修改 Master Spec。

---

# 107. Agent Definition of Done

任何 Task 完成至少满足：

代码可以编译。  
相关测试通过。  
没有关闭测试。  
没有新增明显警告。  
没有擅自修改公共接口。  
文档已更新。  
Benchmark 未明显回退。  
任务 Scope 外没有大规模修改。  
提交 Change Summary。  
说明已知限制。

---

# 108. 第一批真正应该创建的 Issue

不要第一天开发“史官”或“战争”。

第一批严格按照：

CHRON-001 Rust Workspace  
CHRON-002 Godot Project  
CHRON-003 Godot-Rust Bridge  
CHRON-004 Stable EntityId  
CHRON-005 Simulation Clock  
CHRON-006 Scheduler  
CHRON-007 Headless Runner  
CHRON-008 10K Dummy Benchmark  
CHRON-009 Structured Event  
CHRON-010 Developer Metrics  
CHRON-011 128×128 Tile Renderer  
CHRON-012 Snapshot Prototype  
CHRON-013 SQLite Event Prototype  
CHRON-014 Architecture Spike Report

完成 CHRON-014 以前：

禁止正式开发 NPC 心理、文明或 NLG。

---

# 109. Architecture Spike 的最终问题

它必须回答：

Rust ↔ Godot 边界是否足够高效？

bevy_ecs 是否继续使用？

Event Store 每秒能写多少事件？

Snapshot 多大？

10K Dummy Entity 内存是多少？

Godot 128×128 Tile 性能如何？

Headless Simulation 比 Real-time 快多少？

M5 16GB 的实际可用内存预算是多少？

如果结论不理想：

此时修改架构。

这是成本最低的时候。

---

# 110. 最大项目风险

风险一：

Simulation 做了很多，但历史无聊。

解决：

每个 Phase 都运行 Emergence Test。

风险二：

历史数据库爆炸。

解决：

Significance + Snapshot + Hot/Warm/Cold。

风险三：

AI Agent 把架构改乱。

解决：

Master Spec + ADR + Task Scope。

风险四：

NPC AI 太死板。

解决：

Utility + Memory + Personality + Values，而不是无限 if。

风险五：

LLM 成为性能瓶颈。

解决：

完全可关闭。

风险六：

NLG 文本机械重复。

解决：

NLG 独立作为核心系统长期打磨。

风险七：

功能无限膨胀。

解决：

严格 MVP Freeze。

---

# 111. MVP 成功的最终验收故事

创建一个世界。

运行 200 年。

玩家在 Archive 看到：

“第七十六年：河谷战争爆发。”

点击战争。

发现原因来自：

连续粮食短缺  
→ 北方人口南迁  
→ 土地争端  
→ 两个家族关系恶化  
→ 边境冲突  
→ 战争。

点击某个士兵。

看到：

出生于农民家庭。  
父亲死于冲突。  
形成对北方人的怨恨。  
成年后成为铁匠。  
好友参军。  
最终自己参战。  
战斗中左臂受伤。  
战争后离开故乡。  
晚年成为地方官员。  
81岁死亡。

打开他的 Memory：

发现他认为：

“北方军杀死了父亲。”

但是进入 Omniscient：

发现真正杀死父亲的是本国逃兵。

再打开一百年后的史官著作。

史官引用：

士兵自己的回忆录。

所以后世史书记载：

“北方士兵杀死其父。”

玩家点击这句话：

可以看到它依据的是：

一份存在错误记忆的第一手材料。

如果这个闭环自然产生，而不是预写剧情：

**MVP 成功。**

---

# 112. 项目最终定位

这款游戏最重要的竞争点不应该是：

“比 Dwarf Fortress 模拟更多器官。”

也不应该是：

“每个 NPC 都使用 LLM。”

真正值得追求的是：

> **一个拥有真实因果、主观记忆、信息偏差、史料传播和历史书写过程的可观察世界。**

Dwarf Fortress 擅长：

模拟发生了什么。

Palimpsest 应进一步回答：

**为什么发生？**

**谁知道？**

**他们如何记住？**

**他们如何记录？**

**后人最终认为发生了什么？**

这才应该成为整个项目不可替代的核心。