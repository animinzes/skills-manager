# Agent Manager 开发计划（详细版）

> 版本：v2.2（2026-09-29 更新：合并上游 v1.40.1〔99 提交〕——上游自带 zcode 适配器〔本地保留共享池增强〕与批量同步 UI〔本地保留服务端整批命令并接管其调用〕；subagent 管理随 M7 提前实现）
> 分支纪律沿用：`main` 镜像上游，`skill-control` 为个人工作分支，每批次短特性分支 + 可逆提交，不建 PR
> 工作量标记：S = 半天内，M = 1~2 天，L = 3 天以上（含调试与测试）
> 构建铁律：出 exe 必须走 `npm run skill-control:build`（裸 cargo build 的 exe 前端加载坏）；改前端后该命令会自动重跑 npm build

## 批次总览（顺序即依赖顺序）

| 批次 | 名称 | 一句话 | 工作量 | 依赖 |
|---|---|---|---|---|
| M0 | 启用摸底 | 裸跑现有 exe，首次导入 278 技能 | S | 无（零代码） |
| M1 | 适配器补齐 | zcode 适配器 + 共享池优先 | M | M0（要库先建起来） |
| M2 | **管理操作完备（核心）** | 批量启停/部署/删除 + 变更计划 + 部署矩阵 | L | M1 |
| M3 | 套件多组件 | 成员类型扩展 + 收编 + 三件套 | L | M2 |
| M4 | 接管 npx skills | lock 迁移 + add/list/remove/update | M | M1（与 M2/M3 并行可行） |
| M5 | 市场接入 | 策展白名单 + skills.sh 过滤层 | M | M4（安装通道复用） |
| M6 | 质量评级与清理 | 分级 + 重复对 + 漂移 + 清理建议 | M | M2 |
| M7 | 机制扩展 | 四类纯文件资产 → MCP/hooks | L | M2 |
| M8 | 健康与接口收尾 | 健康检查四组 + 本地 API/MCP + 界面完善 | L | M3/M6/M7 |

建议节奏：M0→M1 连续做完（一周内可见底子好坏）；M2 是核心，做完后用户日常管理动作已全部可走本应用；M3/M4 可并行；M5~M8 按需排。

---

## M0 启用摸底（零代码，信息量最大）✅ 已完成（2026-09-28，报告见 `M0-摸底报告-2026-09-28.md`：通过；中央库已建 D:\SkillControl；导入语义=复制入库；发现 common.refresh i18n bug 已随 M1 修复）

**目标**：验证 8 月成果值不值得继续投入；拿到第一份真实数据下的 bug 清单。

任务：
1. 启动 exe，设置中央库路径 `D:\SkillControl`（默认 C 盘违反硬约束），关闭 GitHub 远程备份
2. 首次扫描导入 `~/.agents/skills`（278 技能）
3. **核实导入语义**：确认是引用原目录还是复制进库（若复制 → 278 份副本会破坏"正本唯一"，记录并作为 M2 设计输入）
4. 摸底记录：界面可用性、扫描耗时、导入错误、治理字段编辑体验、Registry 页面四区表现
5. 产出：一份摸底报告（bug 清单 + 继续投入判断）

**验收**：278 技能入库不报错；中央库在 D 盘；导入语义已查明并记录。
**风险**：从未跑过真数据，撞 bug 概率高——这一步就是用来撞的；数据全在原目录，重导无损失。

## M1 适配器补齐（zcode + 共享池）✅ 代码完成（2026-09-28：zcode 适配器入库，`.zcode/skills` 部署 + `.agents/skills` 附加扫描，436/436 测试过；common.refresh 三语言补齐）

**目标**：管到用户主力环境 ZCode。

任务：
1. `tool_adapters.rs` 新增 zcode 适配器：发现/部署目录 = `~/.zcode/skills` + `~/.agents/skills`（后者与 Codex/OpenCode 共享）
2. 部署层实现**共享池优先**：默认部署目标 `~/.agents/skills`，五端专属目录按需二段分发
3. 适配器单测：路径发现、部署、取消部署、链接与复制两种模式
4. 顺带评估：qoderworkcn/qwenworkcn/workbuddy/cc-switch 四个环境是否真在用（按实际使用决定要不要适配，默认不做）

**验收**：向 ZCode 部署一个技能后，ZCode 新会话能发现该技能；取消部署后不再发现。
**风险**：S；上游适配器接口已成熟，照葫芦画瓢。

## M2 管理操作完备（核心批次）◐ 第一增量完成（2026-09-28 提交 95492c0：多选批量部署/取消部署到任意 agent——新命令 apply_skills_to_agents 走 apply_skills_to_tools 整批计划+整批拒绝语义，BatchDeployDialog 选目标 agent，tsc/eslint/436 测试全过）

**目标**：把 REQUIREMENTS 第二节八类管理动词全部做成可操作——这是产品的主轴。

任务：
1. **批量操作**：多选 → 批量启用/停用/部署/导出/删除；批量删除强制走变更计划 + 逐项确认
2. **变更计划**（并入旧 Phase 4）：批量写先出计划（影响资产/套件/agent/路径、预期文件操作、恢复方式）→ 确认 → 执行 → 逐项记录结果；统一走一个共享 mutation 服务（复用现有审计日志）
3. **部署矩阵视图**：资产 × （共享池 + 五端）状态矩阵，支持从矩阵直接切换启停
4. **部署模式管理**：链接 vs 复制按目标记忆；提供"全部转链接/转复制"的批量迁移计划
5. **组织批量编辑**：分类/标签/分级多选批量改（每次走修改历史）
6. **界面强化**（旧 Phase 6 首批）：多字段筛选（类型/标签/分级/来源/部署状态）、批量选择、资产详情页治理编辑器扩展到新字段
7. 若 M0 查明导入为复制语义：设计并实现**引用模式导入**（登记不搬文件），消除第二正本问题

**验收**：REQUIREMENTS 2.1~2.7 的动词逐项可演示；任一批量删除可从快照恢复；矩阵视图一屏看清五端状态。
**风险**：L；变更计划是新的核心抽象，先做最小闭环（部署/删除两类动作）再扩展。

## M3 套件多组件模型

**目标**：套件真正能表达 Claude Scholar 这种"技能+命令+子代理+钩子"的复合安装。

任务：
1. schema 扩展：suite 成员条目加 type（skill / command / subagent / hook / mcp / settings-mod / template / script / in-process）+ path + 所有权
2. **受管路径清单**：套件安装时记录全部写入路径 + 来源 + 版本；清单文件随套件清单存 registry/suites/
3. **收编（adopt）**：对着现成安装生成清单并登记（不重装）——首测对象 = 本机已装的 Claude Scholar（60 技能 + commands/agents/hooks）
4. **覆盖前快照** + **settings 合并式修改**：修改宿主配置只增量写入、记录 diff、卸载按 diff 还原；CLAUDE.md 冲突装侧车
5. 嵌套上游 UPSTREAM 指针（obsidian-skills 案例登记）
6. 套件完整性校验 + 导入导出（补齐旧 Phase 2 后半）
7. 套件升级按策略执行（atomic / installer-managed 等）

**验收**：Claude Scholar 完整登记为一个套件（含非 skill 成员）；停用该套件后五端目录按清单还原；重新启用可恢复。
**风险**：L；settings 合并式修改要处理五种宿主格式，先做 Claude Code 的 settings.json 一种打通模式。

## M4 接管 npx skills

**目标**：装外部技能的日常入口换成 Agent Manager。

任务：
1. `.skill-lock.json`（v3）读取与迁移：9 项登记导入，来源/哈希/时间戳保留
2. `add`：GitHub 仓库与 monorepo 子路径安装（复用上游 skills.sh 导入协议与 Git 导入）；装后默认不部署，点名才分发
3. `list` / `remove` / `update`：列状态、卸载清登记、按 skillFolderHash 查更新
4. **恢复**：从登记文件重建全部安装（对应 experimental_install）
5. `init`：新技能骨架创建（服务自建技能）
6. CLI 对齐第十节目标形态，写 `agent-manager` 专用 SKILL.md（agent 可自动注册）
7. `npx skills` 不卸载、lock 文件保留作迁移凭据；切换后新安装只走本应用

**验收**：lock 9 项迁入后 list 状态正确；`add anthropics/skills` 子路径成功装一个；remove 后目录与登记都干净；恢复流程从零重建。
**风险**：M；skills.sh 协议上游已支持，主要工作在 CLI 与迁移。

## M5 市场接入（质量优先）

**目标**：找技能也只开 Agent Manager。

任务：
1. **策展源白名单**：内置可信 GitHub 源清单（anthropics、microsoft、cloudflare、stripe、trailofbits…，取自 VoltAgent/awesome-agent-skills 官方分区）；白名单编辑界面（增删改）；从白名单一键安装
2. **skills.sh 过滤层**：搜索结果按安装量阈值 + 官方来源标记 + 审计状态过滤排序，未过过滤的折叠显示并标警示
3. 发现推荐流：解析 awesome 列表生成分类推荐（只读，可后置）
4. P2 中文三家（腾讯 SkillHub / ClawHub / 魔搭）API 适配，默认关闭，用户开启才出现
5. 明确不接：skillsmp、skill.market

**验收**：白名单增删一个源并成功安装；skills.sh 搜索默认视图无零信息条目；垃圾站不可达。
**风险**：M；skills.sh 无公开文档化的排序 API，过滤层做在本地结果上。

## M6 质量评级与清理

**目标**：产品化 4 月那条线——"留的每一项都有理由"。

任务：
1. 分级模型：A/B/C/D + 评语 + 确认状态机（unreviewed → suggested → confirmed → locked；扫描不覆盖 locked）
2. **批量建议分级**：按来源（官方白名单=A 候选）、使用痕迹、描述质量、重复情况生成建议
3. **重复对识别**：名称相似度 + 描述语义 + 功能关键词，输出"保留谁/合并谁/删谁"建议名单
4. **漂移报告**：共享池与五端目录 vs 登记哈希的比对视图（失效链接/分叉/孤儿）
5. 清理建议三件套（删除/合并/归档名单）→ 逐项确认执行（走 M2 变更计划）
6. 首次全库跑一遍：对 278 技能产出第一份分级+重复+漂移报告

**验收**：全库报告产出；locked 项在重扫后不变；一条清理建议从建议到确认到执行全程留痕可回滚。
**风险**：M；分级建议规则先简单（来源+重复），避免过度智能。

## M7 机制扩展（skill 之外的资产）◐ subagent 管理已实现（2026-09-29：v10 schema `agent_definitions`/`agent_definition_targets`，中央库 `agents/` 目录，Claude Code .md + Codex .toml + OpenCode .md 三端，导入收编/部署/取消部署/删除全带所有权保护与审计，新"子代理"页面含扫描收编）

**目标**：从技能管理器长成 agent 扩展资产管理器。

任务（第一版，纯文件四类）：
1. 资产模型扩展：skill 之外登记子代理定义（.md/.toml）、斜杠命令（commands/prompts）、记忆文件（CLAUDE.md/AGENTS.md/GEMINI.md 只登记备份）
2. 各端发现目录接入这些机制的扫描与部署（矩阵视图加维度）
3. 记忆文件的特殊规则：只读登记 + 快照备份，永不代写内容

任务（第二版，配置写入类）：
4. MCP 配置合并式写入五端（claude.json / config.toml / zcode cli\config.json / gemini settings.json / opencode.json），每端一个成功 + 一个回滚用例
5. hooks 成组部署（脚本 + 注册一起）

后置：插件市场注册表管理、permissions、statusline、Gemini commands（本机未用）。

**验收**：一个子代理定义与一个命令成功部署到 Claude Code 与 Codex 并被识别；MCP 条目写入后宿主可启动、回滚后配置还原。
**风险**：L；写宿主配置文件错了会影响 agent 启动——合并式修改 + 自动备份 + 回滚用例是硬要求，五端格式各异逐一适配。

## M8 健康与接口收尾

**目标**：常态体检 + agent 可直接查询。

任务：
1. 健康检查四组完整（结构/部署/依赖/套件），仪表盘汇总 + 证据与修复建议；自动探测只跑无副作用 probe（凭据只查存在性）
2. 本地 API / MCP 适配器（旧 Phase 5）：先只读（inventory/health 查询），再鉴权变更端点
3. 界面完善（旧 Phase 6 收尾）：依赖图视图、变更计划审查页、执行历史页、治理/套件清单导入导出
4. `agent-manager` SKILL.md 与 MCP 打通后，agent 可完成"查询本机有什么能力"的场景

**验收**：仪表盘能指出缺失依赖与漂移并给证据；Codex/Claude Code 经 MCP 查询资产清单成功。
**风险**：L；接口安全（本地鉴权）需要设计，只读先行。

---

## 暂不做（明确出界）

- 追上游 88 提交（用户已拍板暂不追；将来想追时 merge 无功能冲突）
- qoder / qoderworkcn / qwenworkcn / workbuddy / cc-switch 适配器（按实际使用再定）
- 25 agent 全矩阵部署（收敛为共享池 + 五端）
- 进程内套件组件的深度管理（oh-my-opencode 类只登记一行清单）
- myskill 目录迁移（用户明示先不动，将来统一合并时处理）
- skillsmp / skill.market 等零审核市场

## 开发规则（沿用）

- `main` 镜像上游；`skill-control` 个人工作分支；每批次短特性分支 + 可逆提交，本地检查过后并入
- 不建 PR / Issue；schema 变更必带迁移与级联测试；依赖升级与功能分开提交
- 每批次结束更新本文勾选状态与 REQUIREMENTS 验收记录
