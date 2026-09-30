# Agent Manager 需求定稿

> 版本：v2.1（2026-09-28 定稿并强化"管理"主轴，取代 2026-08-13 英文初版，初版内容已并入）
> 产品名：**Agent Manager**（旧名 Skill Control 作废；构建标识 `com.animinzes.skillcontrol` 与隔离构建配置不变）
> 性质：animinzes/skills-manager 个人 fork，纯自用，不向上游提交 PR
> 配套文档：ROADMAP.md（v2.1，与本文件同日重写的详细开发计划）

## 一、产品定位

一个本地桌面应用，统一**管理**多个 coding agent 的全部可文件化扩展资产——skill、子代理定义、斜杠命令、hooks、MCP 配置、记忆文件——以及它们组成的套件。本地文件始终是事实来源，应用记录文件系统表达不了的关系（归属、依赖、部署状态、质量评级、变更历史），并对这些资产执行完整的管理动作。

三个"替代"目标：

1. **替代 `npx skills`**（vercel-labs/skills CLI）：装、删、列、搜、更新外部技能的日常入口全部改用本应用；
2. **替代各应用内嵌技能市场**：发现外部技能走本应用的策展源与市场接入；
3. **替代手工目录维护**：跨 agent 的部署、启停、漂移检测、清理由本应用管理。

## 二、管理功能（最重要的功能）

管理的完整动词集。一切开发以"能不能把管理动作做完"为检验标准——只登记不能操作的工具没有价值。

**登记纪律四条（2026-09-29 用户拍板，优先级高于任何方案提议）**：
1. **登记先行**——全部技能先登记入库，不等分类、不等套件组装（已执行：四端收编完成，库内 319 条）；
2. **分发只允许复制**——任何部署永远是 copy 模式，禁用符号链接/junction 部署；
3. **登记库隔离不变式**——中央登记库（D:\SkillControl）是纯登记区，**永不允许成为任何 agent 的扫描/发现/消费目录**：库里保留着「暂无任何 agent 使用、要用时才现成打开」的技能；
4. **分类、套件组装、分发最后进行**——顺序固定为 M6 质量评级/重复对 → M3 套件 → 部署。

### 2.1 盘点管理（知道有什么）

- 扫描五个 harness 的全部机制目录 + 共享池，导入建库；
- **收编（adopt）**：识别已存在的套装安装、npx skills 安装、手工目录，登记进库而不重装；
- 迁移 `npx skills` 的 `.skill-lock.json`（9 项登记直接迁入）；
- 资产身份稳定：目录名/声明名可变，靠来源+路径+内容哈希识别同一资产；同名不同内容不自动合并。

### 2.2 组织管理（分门别类）

- 主类型五分类（capability / tool_guide / integration / workflow / governance）+ 任意自定义标签；
- 内置分类 **`animiznesのworkflow`**：用户自有技能全部归入（名称一字不差，含日文の）；
- 套件归属：一技能可属多套件，成员关系不改主类型；
- 质量分级 A/B/C/D + 评语，带确认状态（unreviewed → suggested → confirmed → locked；系统建议必须经用户确认，扫描不得覆盖 locked）；
- **批量编辑**：分类、标签、分级支持多选批量修改。

### 2.3 部署管理（控制每个 agent 看到什么）

- 逐 agent 的启用 / 停用（部署 / 取消部署）；
- **批量操作**：多选批量启停、批量部署、批量导出、批量删除（删除走变更计划+逐项确认）；
- 部署模式可选：符号链接（改一处全生效）或复制（独立副本），记录每个目标用的哪种；
- 套件整体部署 / 停用 / 升级，成员失败自动回滚；
- 部署矩阵视图：资产 × 五端 + 共享池的启用状态一目了然；
- 共享池优先：默认部署目标为 `~/.agents/skills`（Codex/OpenCode/ZCode 共同消费），按需再分发到各端专属目录。

### 2.4 更新管理（保持不过时）

- 按来源（Git 仓库 / 子路径 / 市场）检查更新，按内容哈希判断漂移方向；
- 套件升级按策略执行（atomic 整体更新 / member-independent / installer-managed 只允许安装器动 / pinned 只报告）；
- 更新走变更计划：预览哪些文件会变、哪些部署要跟着刷。

### 2.5 清理管理（回答"越多越好吗"——不，留的每一项都要有理由）

- 无效资产识别：失效链接、空壳目录、SKILL.md 不可解析；
- 漂移检测：共享池/各端目录与登记哈希比对，分叉、漂移、孤儿一目了然；
- 重复对识别：名称不同功能相同的组（实证案例：pdf vs PDF Processing Pro、ppt-creator/slides/pptx、spreadsheet/Excel Analysis/xlsx）；
- 清理建议三件套：建议删除名单、建议合并名单、建议归档名单；
- **安全红线：清理只出建议，删除/覆盖永远逐项确认；清理授权跟着当前任务走，历史盘点清单不构成新授权。**

### 2.6 发现与安装管理（进新东西的唯一入口）

- 策展源白名单订阅（见第五节）+ 市场搜索 + 一键安装入库 + 部署；
- 复刻 `npx skills` 动词集（见第四节）；
- 安装默认不部署：先入库，用户点名才分发。

### 2.7 变更与恢复管理（敢动手是因为能撤回）

- 批量写操作先出变更计划（影响面、前置条件、恢复方式），确认后执行；
- 覆盖前快照备份；
- 追加式修改历史（changes.jsonl，只增不改不删，含修改前后数据）；
- 从登记文件/lock 恢复全部安装（对应 `npx skills experimental_install`）；
- registry 文件可重建 SQLite——重建库/换机不丢登记。

### 2.8 监控管理（常态体检）

健康检查四组（结构 / 部署 / 依赖 / 套件），每条结果带证据与修复建议，自动探测只跑无副作用 probe；漂移与依赖缺失汇总为仪表盘。

## 三、管理对象

### 3.1 资产类型（六类可文件化资产）

| 资产 | 形态 | 说明 |
|---|---|---|
| Skill | 目录 + SKILL.md | 遵循 Agent Skills 规范（agentskills.io） |
| 子代理定义 | .md（Claude Code / OpenCode）或 .toml（Codex） | frontmatter/配置描述一个 subagent |
| 斜杠命令 / prompt | .md（commands / prompts 目录） | Claude Code commands、Codex prompts |
| Hook | 脚本 + 注册（hooks.json / settings 引用） | 成组管理：脚本与注册项一起部署/回滚 |
| MCP 配置 | 各宿主配置文件中的 server 条目 | 写入必须合并式修改（见 3.5） |
| 记忆 / 指令文件 | CLAUDE.md / AGENTS.md / GEMINI.md | 只登记与备份，不代写内容 |

### 3.2 管理范围：五个常用 harness

只管用户常用的五个，其余环境（qoder / qoderworkcn / qwenworkcn / workbuddy / cc-switch 等）暂不纳入。

| 机制 | Claude Code | Codex | ZCode | AGY (Gemini) | OpenCode | 批次 |
|---|---|---|---|---|---|---|
| skills | ~/.claude/skills | ~/.codex/skills + **~/.agents/skills** | ~/.zcode/skills + 插件携带 | config/skills + skills | **~/.agents/skills** + 兼容读 .claude/skills | 第一版 |
| 子代理 | agents/*.md（139个） | agents/*.toml（169个） | — | — | agents/*.md | 第一版 |
| 命令/prompt | commands/*.md（45） | prompts/*.md（20） | — | （未用） | commands/*.md | 第一版 |
| 记忆文件 | CLAUDE.md | AGENTS.md | — | GEMINI.md | instructions | 第一版 |
| MCP 配置 | ~/.claude.json | config.toml | cli/config.json | settings.json | opencode.json | 第二版 |
| hooks | hooks/ + settings | — | — | 有 hook 系统 | — | 第二版 |
| 插件+市场 | plugins/ | — | cli/plugins/ 注册表 | config/plugins | plugin | 后置 |

**明确排除**（不可文件化，永不管理）：各 harness 的 SQLite 数据库、会话记录、cron 自动化、凭据缓存。

### 3.3 Resource（资源注册表）

外部工具与服务作为独立对象登记：`cli`（OpenCLI、node、python…）、`service`（AMiner、翻译 API…）、`mcp`、`runtime`。技能与套件声明依赖；Resource 记录版本探测方式与存储策略（system_bound 不动 / relocatable 可迁 D 盘 / user_selected 需确认 / unknown 只读检查）。

### 3.4 Suite（套件）

多类型成员的组合包。成员类型全集（实证）：skills、斜杠命令、子代理、hooks、MCP server、settings 修改、CLAUDE.md 注入、templates、安装脚本、**进程内组件**（如 oh-my-opencode 的 TS hooks——无法当文件管，仅登记一行清单）。

套件定义：版本、必需/可选成员、成员版本约束、Resource 依赖、支持的 agent、安装器所有权、升级策略、整体健康。

**管理机制三件套**（借鉴 Claude Scholar 实证做法）：受管路径清单（声明所有权、支持收编）；覆盖前快照备份；settings 合并式修改（保留用户字段、记录增量、卸载按增量还原；CLAUDE.md 冲突装侧车文件）。

**嵌套上游**：vendored 第三方技能记录 UPSTREAM 指针（案例：Claude Scholar 带入的 18 个 obsidian-* 实为 kepano/obsidian-skills）。

**安装器四形态**：脚本安装器（setup.sh / pipx / npx init）、plugin marketplace（plugin.json）、注册表 CLI（ClawHub 式）、npm 进程内插件（只登记）。

**本机需登记的套件**（首批实测对象）：

| 套件 | 本机痕迹 | 安装器形态 |
|---|---|---|
| Claude Scholar | 60 个 source-command-* 技能、commands/agents/hooks | 脚本（setup.sh，有 manifest） |
| SuperClaude | commands 30 + agents 20 + skills 6 + hooks + 可选 8 MCP | pipx CLI |
| obsidian-skills | 18 个 obsidian-*（Claude Scholar vendored） | 嵌套上游 |
| OpenCLI 技能组 | 5 个技能 | npx skills（.skill-lock.json 登记） |
| AMiner 技能族 | 4 个 aminer-* | 手动安装 |
| AI-research-SKILLs | 曾研究（2026-04），vault 有残留笔记 | git（历史对象） |

### 3.5 Preset

用户临时勾选的批量部署组，无版本无依赖（沿用上游语义）：Preset 管批量选择，Suite 管组合关系。

## 四、接管 npx skills

**接管对象**：vercel-labs/skills CLI（skills.sh 官方命令行，本机 npx 缓存 v1.7.0）。

| npx 命令 | Agent Manager 对应能力 |
|---|---|
| add \<package\> | 从 GitHub 仓库 / monorepo 子路径安装入库，按需部署 |
| list | 列出已装技能及部署状态 |
| find [query] | 搜索市场（第五节） |
| remove | 卸载并清理登记 |
| update | 按内容哈希检查来源更新 |
| use \<package\>@\<skill\> | 免安装临时使用（后置） |
| experimental_install | 从登记文件恢复全部安装 |
| init [name] | 创建新技能骨架（服务自建技能） |

**迁移资产**：`C:\Users\animi\.agents\.skill-lock.json`（v3：source / sourceType / sourceUrl / skillPath / skillFolderHash / installedAt / updatedAt / dismissed / lastSelectedAgents），现 9 项：find-skills、xiaohongshu-cli、OpenCLI 5 件、humanizer-zh、image-to-editable-ppt。

**部署范围收敛**：lastSelectedAgents 从 25 个 agent 全选收敛为"共享池 + 常用五端"。`npx skills` 本身不卸载，登记文件保留作迁移凭据。

## 五、市场与发现（质量优先）

**原则（用户明示）**：拒绝"技能很多但大部分是垃圾"的市场。

| 优先级 | 渠道 | 接入方式 |
|---|---|---|
| P0 | **策展源订阅**：可信 GitHub 源白名单（anthropics/skills、microsoft、cloudflare、stripe、trailofbits 等官方仓库，参考 VoltAgent/awesome-agent-skills 官方分区维护） | 上游 Git 导入，白名单用户可增删 |
| P0 | anthropics/skills 官方仓库 | 默认内置源 |
| P1 | skills.sh（Vercel） | 复用上游协议 + **本地过滤**（安装量阈值 + 官方来源 + 审计状态），因其靠遥测上榜无策展 |
| P1 | VoltAgent/awesome-agent-skills | 发现推荐流 |
| P2 | 中文三家：腾讯 SkillHub（skillhub.cn）、ClawHub、魔搭 ModelScope Skills Central | API 适配，可选发现源 |
| 不接 | skillsmp.com（零审核聚合）、skill.market（空壳） | 明确拒绝 |

## 六、质量评级（并入 2.2/2.5 执行）

A/B/C/D 四级 + 确认流 + 重复对识别 + 漂移报告 + 清理建议三件套。溯源：2026-04-04 用户实操线的产品化。

## 七、自有技能管理

- 自有技能归入 `animiznesのworkflow` 分类。当前清单：animinzes-brain、lab-report、live-lecture-notes、time-system、relationship-manager、note-style、note-outlinize-clean、pdf-formatting。
- **myskill 目录（C:\Users\animi\myskill）**：用户定性"当时的汇总想法，文件形式汇总不正确、不易管理"——不依赖、不迁移、暂不动；待本应用可用后统一合并各 agent 自有技能时再处理。distribute_skill.ps1 在新部署层验证可用前保持现状。

## 八、治理元数据、存储与健康

**治理字段**（每资产可记录）：主类型、来源（official / third_party / local / unknown；套件成员关系单列，来源不取值 suite）、所有者与维护者、生命周期（experimental / active / deprecated / archived）、审查风险、支持 agent 及级别、Resource 依赖、权限要求（文件/网络/命令/账号/密钥）、维护备注。治理信息以文件为准（registry 可重建 SQLite 投影）。

**存储模型**（中央库 = D:\SkillControl，C 盘约束见第九节）：

```text
D:\SkillControl\
├── skills\                         技能本体（含 .skills-manager 内部同步元数据）
├── registry\
│   ├── skills\                     资产身份、分类、标签、治理
│   ├── suites\                     每套件一份完整清单
│   └── tools\index.json            Resource 汇总
├── history\changes.jsonl           追加式修改历史
└── skills-manager.db               可重建的本地查询库
```

**健康检查**四组：结构（SKILL.md 可解析、引用完整）、部署（目标存在、哈希一致、链接有效、漂移）、依赖（Resource 探测，凭据只查存在性）、套件（成员完整、版本一致）。每条结果带证据与修复建议。

**变更模型**：批量写先出计划（影响的资产/套件/agent/路径，预期创建/更新/链接/删除，权限影响，前置条件，恢复方式），确认后执行并逐项记录结果。账号写入、破坏性删除、大范围部署必须单独显式确认。

## 九、非功能需求与硬约束

- 用户 2026-08-13 硬约束原话（永久有效）："不要 pr！！我只想做好了先自己用！""尽量不要在 C 盘""仅限于完全可以移动到D盘的情况！！""不要产生影响！"
- 中央库默认 C 盘 `~/.skills-manager` 违反约束：首次导入前必须指到 `D:\SkillControl` 并关闭 GitHub 远程备份。
- Windows 路径、junction、symlink 自动化测试覆盖；数据库迁移事务且幂等。
- 个人构建沿用隔离配置（独立 identifier），构建缓存与产物在 D 盘。
- **构建现状（2026-09-28）**：`.cargo-target` 已在一次越界清理事故中删除；现存 exe = `D:\Projects\skill-control\skills-manager.exe`（顶层）。下次改代码为全量重编，须预留时间。
- 写操作安全红线：删除/覆盖/批量部署逐项确认；清理授权跟着当前任务走。

## 十、接口

GUI、CLI、本地 API 与未来 MCP 适配器调用同一套 Rust 领域服务；文件系统与数据库变更全部在 Rust 后端。CLI 目标形态（供 agent 自动调用）：`register <path> --dry-run/--apply`、`suite register`、`inspect <id> --json`、`deploy <id> --agent <key>`，配一份 `agent-manager` 专用 SKILL.md 指导 agent 完成注册（旧版 skill-control-registry 的落地）。

## 十一、验收标准

**已达成**（2026-08-13）：schema v8 治理基础 8 项验收全过；Registry 页面与套件整体部署/回滚 10 项验收全过；Rust 435/435 测试。

**新增验收（按 ROADMAP 批次对应）**：

1. **M0 首次导入**：278 技能入库不报错；引用或可控复制（不产生第二份正本）；中央库在 D:\SkillControl。
2. **M1 zcode 适配器**：`.zcode/skills` 与 `.agents/skills` 登记为 ZCode 发现/部署目录；部署后 ZCode 能发现。
3. **M2 管理操作完备**：批量启停/部署/导出/删除全走变更计划；部署矩阵视图可用；八类管理动词（第二节）逐项可演示。
4. **M3 套件多组件**：Claude Scholar 实测登记，成员含 commands/agents/hooks；收编与受管路径清单可用。
5. **M4 接管 npx skills**：lock 9 项迁入；add/list/remove/update 对 GitHub 源（含子路径）可用；恢复流程可用。
6. **M5 市场**：白名单可增删并安装；skills.sh 搜索带过滤层。
7. **M6 质量评级**：批量建议分级 + 重复对名单 + 漂移报告 + 确认流可用。
8. **M7 机制扩展**：第一版四类纯文件资产可登记部署；MCP 合并式写入五端各有一个成功与回滚用例。
9. **M8 健康与接口**：四组健康检查完整；本地 API/MCP 只读端点可用。

## 十二、需求来源（演化线索）

- **2026-04-04**：首次全库审查 + 漂移清理 + 质量分级；用户根本问题"到底应该如何管理 skill，skill 不应该是能复用的工作流吗？到底是不是（有用的）越多越好？"
- **2026-08-13**：fork（基线 v1.33.1），Phase 1 + Phase 2 前半完成，编译通过后搁置，零使用。
- **2026-09-26/28**：决定启用；定名 Agent Manager；需求定稿为本文件。基线暂不追上游（落后 88 提交全为修复类，无功能冲突）。
