<!-- mimi-known-inventory:start -->
## 已知问题与验收清单

这次盘点 **31 项跟踪事项，不是 31 个 bug**：6 项确认缺陷/已修 UI 问题、4 项实现边界或功能缺口、10 项待验证、5 项功能/调研缺口、2 项未复现外部反馈、3 项历史/测试入口/资源记录，另 1 项钥匙串反馈根因待查。新一轮 QA 又发现 1 项选择框同步缺陷，单列在表后。

版本：本 PR 最新 `8e4432efd2a1dd924fccedd2bbd2ed84cf876ebc`（新增控制条声波同步；桌面/Android CI 全绿）；独立体验候选最新 `d68dd02b6797f04c21ae159f53f1042bec40cdb4`（PC 图示候选不在本 PR；Dock 已集成但原生待验）。b9d124c desktop CI 36837714308 与 Android CI 36837713829 已成功；Linux 包 CI 36838170887 也已成功；artifact 11150278936 实际 ZIP/GitHub摘要/CRC 已核对，AppImage 标签/双语限定复验已过，DEB 与菜单细项待验。此前 a389/db44 成功仅对应旧 head。自动检查不代表真实音频、硬件或权限通过。

**发布优先三项：** ① #83 正常启动/签名对应的钥匙串风险；② #78/#74 拟发布平台的真实采音→字幕链路；③ #91 双语与 #77 历史/导出的针对回归。#93 仅标签的 P2 小修并行于远程 CI 验证；#92 新 Wayland 能力独立推进，不捆绑现有稳定修正。

**来源状态已核对：** #72/#73/#76 已在 main（main `d190877`）；#67/#84/#86/#89 源 head 已在集成分支，保留贡献者 @LLLin000 的来源设计和实现。#85 旧稿仍 HOLD；图示/连续引导只在独立候选，不冒充已批准最终版。此轮不合并、不发布、不关闭来源 PR。

P1 优先解决/验证；P2 下一轮按项推进；P3 调研。每行来源 issue 保持真实状态，未验≠缺陷。

<!-- mimi-auth-release-gate:start -->
### 正常使用与升级授权：发布门槛

**#83 仍开放，不可说整包通过。** 实际正式候选 `b9d124c`（非 dev）已用原证书/Bundle ID/完整 DR 严格验签。用户批准临时替换后，首次正常启动请求“登录”钥匙串认证，访问 `app.yuxino.mimi.credentials.profiles`；未代认证、未改 Key/ACL/TCC、未收费采音。候选正常退出，完整旧正式版恢复并验签，非秘密配置未变化。凭据读取及两次冷启动未通过；弹窗不等于凭据丢失，也未读取真实条目 ACL 断言根因。

后续先协调唯一前台与精确请求者，再固定同一二进制/签名/路径，由用户操作系统认证后做两次冷启动。**成功也只验证这次升级，不承诺未来所有升级免认证。** [可执行步骤与回滚边界](https://github.com/yuxino/mimi/blob/8e4432efd2a1dd924fccedd2bbd2ed84cf876ebc/docs/plans/2026-10-01-formal-candidate-follow-up.md)。正式旧版当前保留；dev 暂停。新 AppImage #93/#91 标签/双语限定结果见下，DEB 与 Mac 菜单细项仍待验。

### 紧凑控制条：声波样式同步

选择 A/B 后，旧控制条仍显示原呼吸灯；`8e4432e` 将已有样式设置传入，不改动效设计。图上部左列旧 `b9d124c`、右列修正；下部三样式×七态静态核对。**Mac 实际无窗口 Chromium/合成状态，非新原生包。** 41 浏览器断言（七态时序/暂停续动/默认系统 reduce/独立开关/切页）及 250 前端测试通过；新 exact-head [桌面 CI](https://github.com/yuxino/mimi/actions/runs/36863412746)/[Android CI](https://github.com/yuxino/mimi/actions/runs/36863412130) 已通过，原生包待验。

![紧凑控制条 before/after 与三样式七状态](https://raw.githubusercontent.com/yuxino/mimi/d9bbe376ef2f1f85fc0c9be3f6751eed7dd38d56/comparison.png)

[原图与完整复现步骤](https://github.com/yuxino/mimi/blob/d9bbe376ef2f1f85fc0c9be3f6751eed7dd38d56/README.md)。
<!-- mimi-auth-release-gate:end -->

### 新 Linux 原生：标签与双语同步

**b9d124c / Linux AppImage / UI-only 合成字幕**。QA 09:47 UTC 同一 PID 102098 用 Ctrl+Shift+B 切三种模式，标签与预览同步；640×136 双语两行完整。两张新原图已查看，并与公开下载逐字节核对。运行 ELF SHA256 `53461c066241eeadb990358217bdb88fe0f3c83af991867324414c9738a1f0be`（QA `/proc/exe` 记录）；artifact 11150278936 的 ZIP/AppImage 哈希已独立核对。

| Before：db44 标签未跟随双语（独立候选） | After：b9 新 AppImage 标签同步 |
| --- | --- |
| ![旧候选标签未跟随](https://raw.githubusercontent.com/yuxino/mimi/2b6c1a961483635cbf2219d829ca3dbf1d4ff8e0/settings-motion-off-bilingual.jpg) | ![新AppImage标签同步](https://raw.githubusercontent.com/yuxino/mimi/3198df14a8bfebf0c0c2b56d435f270b7ee7bdb7/identity-settings-bilingual.jpg) |

![b9实际双语浮窗，两行完整](https://raw.githubusercontent.com/yuxino/mimi/3198df14a8bfebf0c0c2b56d435f270b7ee7bdb7/identity-overlay-bilingual.jpg)

修正原因：快捷键改显示方式后，设置标签应立即跟随。before/after 同字幕页、合成预览/字体；旧窗口含独立引导候选，新生产包不含。**DEB 缺 libwebkit2gtk-4.1 未出现 UI；不算 DEB 通过。** 菜单键盘细项/真实音频/权限未覆盖，#91/#93 不提前关闭。[原图与完整版本边界](https://github.com/yuxino/mimi/blob/3198df14a8bfebf0c0c2b56d435f270b7ee7bdb7/README.md)。

### 钥匙串反馈

| ID / 优先级 | 平台与事项 | 版本/证据 | 当前状态 | 下一步 / issue |
|---|---|---|---|---|
| K01 / P1 | macOS：反复钥匙串提示 | 47ae4c5 / 用户启动反馈 | 根因未解；稳定签名≠Keychain 授权连续 | 先审安全分类补丁；真实授权须用户在场；#83 |

### 已确认问题与实现边界

| ID / 优先级 | 平台与事项 | 版本/证据 | 当前状态 | 下一步 / issue |
|---|---|---|---|---|
| B01 / P1 | 桌面/Linux X11：双语切换隐藏原文 | ad51 原生失败→db44 原生单句循环过 | 3ff08b4+a389 修正已入集成；两行立即可见 | 多句流式/阅读锚点仍待原生；#91 |
| B02 / P1 | 桌面：自动引导保存后关闭 | 58a48ab失败→ad51/db44原生过 | aeae97f修正；候选自动引导保存保持 | 真实首字幕/最终视觉待验；旧#85 HOLD；#82 |
| B03 / P1 | 桌面/历史：导出取消/保存/失败导致Loading状态；重复删除影响较新选择 | 5f2a595组件失败；38/47/087原生分项过 | #84已纳；取消/保存/重试过 | 迟到删除响应/查询分页保留继续验；#77 |
| B04 / P1 | native Wayland GTK3：应用置顶请求/全工作区固定不生效 | Tao0.35.3+GTK3.24.33源码 | 置顶/固定请求为空；当前无layer-shell host | 按GNOME/KDE/wlroots真实backend分验；#92 |
| B05 / P1 | native Wayland：定位、跨屏与控制面板依赖全球坐标 | 当前windows.rs+Wayland协议 | 全球坐标依赖；实际错位未原生复现 | 相对面板、native拖动/resize与双屏；#92 |
| B06 / P2 | Linux GTK3：穿透仍留1×1输入区 | 锁定Tao输入区实现 | 穿透留1×1区；非已复现全窗拦截 | 最小空区/map恢复，验底层窗口点击；#92 |
| B07 / P1 | Windows：Follow-audible默认角色选择与实际声音优先不一致 | #89 2700f6d→6239848/64f8097 | ranked audible选择已修入集成 | 真实Windows角色/静音fallback/拔插；#74 |
| B08 / P2 | 凭据诊断：安全失败类别/数值OSStatus被粗粒度状态隐藏 | 本地a9858e8；当前源码无分类 | 错误类别/OSStatus补丁未纳；不等于ACL修复 | 缓存隔离/恶意Display测试与设置融合；#83 |
| B09 / P2 | Android引导：旧关闭回调会关掉后来打开的引导 | e38199b/889065a/08766d3 | 旧关闭回调修正已纳集成 | exact APK重复打开关闭/迟到回调；#82 |
| B10 / P2 | 桌面/Android：冗余说明、偏暖底色、连接/删除操作拥挤 | 26894ae/d1762cc/861f159等分版本图 | 冗余说明/底色/操作拥挤已小修 | 最新原生图和后续用户视觉反馈；#80 |

### 仍待验证

| ID / 优先级 | 平台与事项 | 版本/证据 | 当前状态 | 下一步 / issue |
|---|---|---|---|---|
| V01 / P1 | macOS/Linux/Windows/Android：真实服务端到端字幕与首字幕完成 | Mac/Linux UI-only、API35合成配置 | 真实provider端到端/首字幕未验 | 用户在场的正常权限/服务会话；#78 |
| V02 / P1 | Windows物理：会议/Teams与设备角色实际路由 | #74贡献者Win11自报，无exactSHA | 非我们独立真机/完整三态复测 | Teams/角色/无PCM/静音/设备恢复；#74 |
| V03 / P1 | Android真机：播放采音、投屏撤回/锁屏、DRM/蓝牙恢复 | API35真实模拟器UI，非物理采音 | Android10/14/15硬件/DRM/蓝牙未验 | 按设备/来源分别验PCM与授权恢复；#78 |
| V04 / P2 | DeepLX/安全配置：旧profile凭据迁移和私人接口认证 | #73 main；197f165高级表单过 | Follow/ASR复用已纳；迁移/私服认证未验 | fake多profile迁移；真实认证另授权；#70 |
| V05 / P2 | 字幕/动效：七态时序、系统reduce、流式多句/长句原生 | 087原生Listening/Paused/样式重启过 | 早期浏览器七态/reduce非最新原生证明 | exact七态/系统reduce/流式长句入口；#87 |
| V06 / P2 | 历史/文件系统：后端磁盘写入失败、reset与迟到响应 | 38/47/087原生取消/路径错误重试过 | 后端disk-error/reset/迟到响应未验 | 隔离合成archive及可控故障入口；#77 |
| V07 / P2 | macOS Dock：Dock/CmdTab/菜单栏/焦点/重启/全屏 | 01b298a自动测试/实际组件 | Dock偏好已纳，默认关；Mac原生未验 | Dock/CmdTab/焦点/全屏/双向重启；#80 |
| V08 / P2 | Linux安装环境：AppImage目录、glibc/发行版、物理声卡与PipeWire | #69权限修复main；包安装smoke | 发行版/FUSE/glibc/物理PipeWire未全验 | 按安装目录和环境记录，不外推Wayland；#66 |
| V09 / P2 | PC首次引导：真实首次字幕/权限跳转、图示视觉最终确认 | #85 HOLD；47/db44独立图示候选 | PC候选未入#88生产；视觉未最终确认 | 视觉/真实权限跳转/首字幕分别验；#82 |
| V10 / P2 | Android新文案：最新三语APK与截图的一致性 | 26894ae文案；e381/087行为修正 | API35图有版本边界；最新APK全流程未验 | zh/en/ja拒绝/撤回/旧安装/重复关闭；#82 |

### 功能与调研缺口

| ID / 优先级 | 平台与事项 | 版本/证据 | 当前状态 | 下一步 / issue |
|---|---|---|---|---|
| F01 / P2 | 字幕字体：字体选择、fallback与跨平台实际效果 | 本地8243b24/54671f5不在集成 | 字体选择与fallback未实现到当前包 | 最小字体review+CJK/缺字/resize；#79 |
| F02 / P2 | 服务费用：用量与费用估算尚未实现 | #81需求；无可靠账单计量 | 用量/费用估算未实现，第三方成本未知 | 计量口径/未知态/官方价格另核；#81 |
| F03 / P2 | 配置/图标设计：完整视觉升级尚未评审/落地 | #80需求；当前仅小幅UI简化 | 完整配置/应用图标升级未完成 | 可比预览确认后落地，不擅改默认；#80 |
| F04 / P2 | ChatMock/通用API：OpenAI-compatible文本翻译与AndroidASR分离尚缺 | #90；实际fork/base/model尚未确认 | ChatMock文本适配未实现；非Realtime音频 | 独立text adapter/ASR；DeepLX不可直替；#90 |
| F05 / P3 | 本地模型：本地ASR+翻译/VAD/分角色需求未实施 | #40/#55调研；#68会议诉求 | 本地ASR/翻译/VAD/角色仍为调研 | 范围/性能/许可证另提可评审方案；#40 |

### 外部反馈与历史记录

| ID / 优先级 | 平台与事项 | 版本/证据 | 当前状态 | 下一步 / issue |
|---|---|---|---|---|
| R01 / P1 | Windows/百度服务：sessionconfiguration.translation rejected | 外部#62，无exact构建/安全错误码 | 百度sessionconfiguration拒绝未复现 | 版本/provider/phase/code；不猜改协议；#62 |
| R02 / P2 | 高强度会议：0.5s停顿断句、上下文不足、多人角色 | 外部#68 Win11长会议自报 | 断句/上下文/角色诉求，无标准复现 | 授权样本或内容无关时序，区分服务能力；#68 |
| H01 / P2 | 多个平台：旧双语/时间戳重叠/窄窗/语言切换更新状态 | closed#59/#48/#44/#43/#42 | 历史修正保留；本次B01独立新缺陷 | 针对回归，不重开他人closed issue；#59 |
| H02 / P2 | Linux UI-only导出：测试停止会清合成archive、原生导出无法验 | 197fixture清档→38 opt-in修正 | 测试入口缺口已修；非生产丢数据 | 新包同包导出；不外推全部fixture；#77 |
| H03 / P2 | 开发Mac：遗留编译/devserver/emulator造成资源压力 | 本任务owner正常清理 | 遗留编译/预览已退，无当前常驻 | 后续串行+结束清理；不动用户应用；#88（记录，无新issue） |

### db44 Linux 原生截图：已通过项与新增问题

Ubuntu 22.04 / X11，实际原生 UI-only，隔离临时配置、合成字幕/服务配置，无真实密钥或 provider。全部为 exact `db44d57`；不是 `b9d124c` / `d68dd02` 修正后的实拍。[原图与范围说明](https://github.com/yuxino/mimi/blob/2b6c1a961483635cbf2219d829ca3dbf1d4ff8e0/README.md)。

**双语立即可见（#91）：** 640×136 / 18px / 居中，反复原文、译文、双语循环，两行不需滚轮。多句流式/历史跟随未测。

![db44 原生双语两行立即可见](https://raw.githubusercontent.com/yuxino/mimi/2b6c1a961483635cbf2219d829ca3dbf1d4ff8e0/bilingual-visible-without-scroll.jpg)

**标签同步缺陷（#93）：** 实际预览已双语，选择框仍仅译文；切页回来标签恢复。这是同一旧包的恢复路径，右图不是修复 after。

<table><tr><th>db44：快捷键改模式后旧标签</th><th>db44：切页重挂载后恢复</th></tr><tr><td><img width="590" alt="旧标签，仅译文与双语预览不同步" src="https://raw.githubusercontent.com/yuxino/mimi/2b6c1a961483635cbf2219d829ca3dbf1d4ff8e0/settings-motion-off-bilingual.jpg" /></td><td><img width="590" alt="切页回来标签恢复，这不是新包after" src="https://raw.githubusercontent.com/yuxino/mimi/2b6c1a961483635cbf2219d829ca3dbf1d4ff8e0/settings-label-after-navigation.jpg" /></td></tr></table>

**PC 候选保存后保持（#82）：** 实际自动引导保存合成配置后仍打开，可继续。页面的“凭据已保存”是内存 fixture，不是安全存储认证证明；视觉仍待用户确认，#85 旧稿继续 HOLD，候选未纳本 PR 生产。

![db44 独立 PC 候选：合成配置保存后引导保持](https://raw.githubusercontent.com/yuxino/mimi/2b6c1a961483635cbf2219d829ca3dbf1d4ff8e0/guide-save-regression.jpg)

### 最新 QA 新增：选择框标签不同步

**#93 / P2 / Linux X11 / db44d57：** 快捷键改为双语后，实际浮层与设置预览已双语，但已打开设置页的模式标签仍为“仅译文”；切通用页再回字幕页才恢复。重复原生截图确认，已另建 issue；受控 value/更新链待排查，不与已通过的双语可见性混成一个问题。修正已推到本 PR b9d124c（候选 d68dd02）：保留按钮焦点，只重建值相关标签，并同步打开菜单的键盘游标。旧菜单外部更新回归会失败，修正后通过；249 前端测试、typecheck、局部 lint、production build 过。jsdom 的旧关闭标签能正常更新，新 AppImage 标签同步与双语可见性已针对通过（见下图）；菜单键盘细项仍待新包原生补验，多句流式/历史跟随仍未覆盖。

**关闭规则：** 仅用户账号 `yuxino` 创建（含代建）的 issue，在集成合并且对应验收解决后逐项评论证据再关闭；外部作者、部分解决、硬件待验、PC 旧稿 HOLD、钥匙串根因未解均保持 open。不使用广泛自动关闭关键词。

下方保留各版本公开 before/after、原图及可操作回归步骤；旧版本、浏览器组件、模拟器、真实原生与待验范围分别标注，图片不冒充当前集成包实拍。
### Issue 收尾矩阵

当前没有可立即关闭的 issue：修正尚未合入/发布，不能因 CI 绿或独立候选存在提前关。验收完成并进入实际交付后，逐项附修复 SHA、版本、平台及实际结果再关闭；此前“仅本人账号创建”的限制继续保留。外部反馈只关联，不批量处置。

| Issue | 作者 | 关闭前还需什么 | 当前处置 |
|---|---|---|---|
| #91 双语可见性 | yuxino | 最终集成包针对回归与合入/交付；db44 原生循环已过 | open，优先候选 |
| #93 设置标签同步 | yuxino | b9 AppImage 标签同步过；菜单键盘细项/Mac与最终合入 | open，限定通过 |
| #77 历史/导出状态 | yuxino | exact 最终包覆盖对应取消/删除状态，合入/交付；disk-error/迟到响应未验需明确 | open，部分验证 |
| #75 脱敏诊断 | yuxino | 最终包复制/内容无关验收与合入/交付，不附带宣称 #83 已修 | open，待最终核对 |
| #74 声音来源/Audio3 | yuxino | Windows 新修正实际路由/无声恢复，保留贡献者反馈边界 | open，硬件待验 |
| #78 跨平台采音 | yuxino | 对应真实采音/恢复，不以模拟器 UI-only 替代 | open，部分实现 |
| #82 引导 | yuxino | Android exact 与 PC 视觉/真实首字幕全部对应解决；#85 仍 HOLD | open，部分解决 |
| #83 钥匙串 | yuxino | 正常启动根因与对应真实授权验收 | open，未解决 |
| #87 字幕/动效 | yuxino | 已纳主体的最终验收；七态/reduce/流式未覆盖 | open，部分验证 |
| #80 配置/图标 | yuxino | 原需求完整解决；目前只小幅简化/Dock | open，部分解决 |
| #79 字体；#81 费用 | yuxino | 功能实施与验收；当前未进入交付 | open，未实现 |
| #66 Linux；#92 Wayland | yuxino | 按专项范围实测；新原生能力未落地 | open，长期/专项 |
| #40 本地模型 | yuxino | 需求确定并实际实施 | open，调研 |
| #70/#90/#62/#68 | 外部作者 | 接口功能/反馈未全部解决；#90 ChatMock 未实现 | open，不自动关 |

来源 PR 不在本次提前关闭；已关闭的历史 issue 保留原状态。发布执行者最终核对矩阵，避免把部分修正当整项解决。

<!-- mimi-known-inventory:end -->

<!-- mimi-release-readiness:start -->
## 发布候选门槛（尚未发布）

当前集成修正 `b9d124c220cb90e43ef98618d13e247d50de03b4`；独立体验候选 `d68dd02b6797f04c21ae159f53f1042bec40cdb4`。此新 head desktop/Android/Linux 包 CI 均已成功，不能沿用 a389/db44 的原生结论；#93 AppImage 标签同步已过，菜单键盘细项/Mac仍待验。

**P0：** 本轮没有确认的凭据泄露、字幕数据损坏或不可恢复删除。发现此类问题立即停止发布。路径选择器报错与 UI-only 清档不能冒充生产数据损坏。

**P1 发布前必须有结果：**

- #83：说明拟发布签名/正常启动的钥匙串情况。固定自签名的指定要求稳定不保证跨构建 Keychain 无提示；现有反复提示根因未解，不能宣布已修。不改 ACL、不重建秘密、不编造 Team ID。
- #78 / #74：拟发布平台正常采音→识别→翻译的安全测试、暂停/恢复/停止和声音来源切换；当前 UI-only、API35 模拟器与 Windows 贡献者无 exact SHA 自报不能代替。真实凭据、权限和计费需用户在场明确授权；没有本地 Windows/Android 物理设备就标未验或缩小交付范围。
- 安装/更新：最终二进制/hash、签名指定要求、正式安装保护、启动和既有配置兼容按实际交付平台核对。开发包验收不替代正式签名/更新链；本轮没有执行发布安装。
- #91：双语显示修正，db44 Linux 原生多次单句模式循环通过；新包仅针对受影响 UI 补回归，流式多句/历史跟随保留未测。
- #77：取消后可再次保存、合成 TXT 一致已有分版本原生证据；后端 disk-error/迟到删除响应仍未测，若交付前发现可复現损坏则升阻塞。

**P2 可延后：** #93 选择框标签（AppImage 同步已过，其他细项待验）、七态/reduce 补验、Dock 原生细项、字体/费用/完整图标设计。#92 新 Wayland 能力独立推进，当前发布不能广告为已经原生 Wayland 全兼容。PC #85 旧稿仍 HOLD；新图示只作明确候选，不默默纳入最终生产。

工作量粗估：#93 AppImage 标签已过；菜单键盘细项按可用测试入口补验；统一 Mac dev 包一次串行增量构建并做 UI-only 回归。真正需要外部条件的是正常凭据/权限/真实 provider 与物理平台路由，不能代用户批准；Wayland/新功能不捆绑当前小修。

这里只准备可核实的发布候选与风险清单；未 merge、tag 或 publish。Refs #88 #83 #78 #74 #77 #91 #92 #93。
<!-- mimi-release-readiness:end -->

## 双语切换：原文和译文一起看（新包待 Linux 原生复验）

切回双语时，把当前这句话滚到可见位置，避免原文藏在窗口上方。手动翻看历史时保留正在看的句子；新字幕仍正常跟随。保留原有句块、定稿与文字动效，不加高浮层来遮掩问题。

**准确版本：** #88 head `a389905b8c7fde8b6f252e992a5d3df3e1548145`；滚动修复为 `3ff08b4` + `a389905`。独立引导候选 `db44d57b48d4bbba8330b4d651b182f9fb9671fb` 已同步这两项及 Dock 偏好，保留引导原生逻辑。候选[Linux 单包 CI 36831890334](https://github.com/yuxino/mimi/actions/runs/36831890334)已全部成功；#88 [跨平台 CI 36831887773](https://github.com/yuxino/mimi/actions/runs/36831887773)与[Android CI 36831887395](https://github.com/yuxino/mimi/actions/runs/36831887395)均completed/success。Linux [工件11147771895](https://github.com/yuxino/mimi/actions/runs/36831890334/artifacts/11147771895)已实际下载，ZIP SHA256 `52a5833b27e06aa3770d2810ee42684fcb5feb456cc0a80b8aaeb63de5fb1c80` 与GitHub一致，CRC及deb/AppImage摘要已核验；deb SHA256 `3f2668500081acba2cfad9e84148c58bdab2bd999aa00739ea3f36f9531fed91`。新 Linux 包原生复验仍待做；本机仍是47安装包，未合并 main、发版或操作真实凭据。

### 同尺寸 before / after

**实际 Mac Chromium 产品组件 + 合成中英字幕；640×136逻辑浮层，1280×272高清图，18px字幕/居中/动效关闭。不是新 Linux 安装包截图。** 使用同一操作「翻译→双语」，after 不需要手动滚轮。

| 前版 ad51bcc：原文被滚到上方 | 新候选 db44d57：两行文字可见 |
| --- | --- |
| [![前版双语仅译文可见](https://raw.githubusercontent.com/yuxino/mimi/5f6612c59f95e9ad12928121579958d470742952/browser-before-ad51.png)](https://raw.githubusercontent.com/yuxino/mimi/5f6612c59f95e9ad12928121579958d470742952/browser-before-ad51.png) | [![新候选双语原文译文可见](https://raw.githubusercontent.com/yuxino/mimi/5f6612c59f95e9ad12928121579958d470742952/browser-after-db44.png)](https://raw.githubusercontent.com/yuxino/mimi/5f6612c59f95e9ad12928121579958d470742952/browser-after-db44.png) |

### Linux 旧包的问题依据

以下两张都是 **ad51bcc 云 Linux 原生 UI-only，640×136，安全合成字幕**；右图只是手动向上滚后的诊断对照，不能作为新修复 after。

| 切到双语：原文不可见 | 同包手动上滚：原文和译文都在 |
| --- | --- |
| [![Linux旧包切双语裁切](https://raw.githubusercontent.com/yuxino/mimi/5f6612c59f95e9ad12928121579958d470742952/linux-ad51-clipped.jpg)](https://raw.githubusercontent.com/yuxino/mimi/5f6612c59f95e9ad12928121579958d470742952/linux-ad51-clipped.jpg) | [![Linux旧包手动上滚显示两行](https://raw.githubusercontent.com/yuxino/mimi/5f6612c59f95e9ad12928121579958d470742952/linux-ad51-manual-scroll-top.jpg)](https://raw.githubusercontent.com/yuxino/mimi/5f6612c59f95e9ad12928121579958d470742952/linux-ad51-manual-scroll-top.jpg) |

**已测：** 实际 Chromium 复现旧版裁切并验证新版两行文字全部可见；原文/译文/双语反复切换、新确认字幕跟随、真实滚轮阅读历史后新字幕不拉回、减少语言行后保留同一句均通过。新增8项有意义的回归（3个React组件 + 5个滚动控制器）；最终候选完整263测试/41文件、typecheck/build、Rust fmt与diff检查通过；lint0错误/1既有SoftwareUpdate警告。

**新包回归：** 隔离 UI-only、640×136/18px → 开始合成字幕 → 翻译切双语，无滚轮两行均可见 → 反复切模式 → 多句时上滚读历史、继续字幕保持位置 → 回末尾继续跟随 → 动效开/关、暂停恢复、尺寸变化。新 Linux WebKit 原生待做；七态/系统减少动态效果/真实音频与认证仍未覆盖。Mac Dock/Cmd-Tab 原生也仍待验。

**ad51 原生结果补正：** 自动首次引导 → 实际合成表单保存 → 引导保持 → 声音/字幕步骤、沉浸进入退出/停止/稍后/重开、中英日已通过；该轮双语浮层失败正是本节所修。旧包通过范围不会冒充新包全部通过。

[固定原图、步骤、版本与校验](https://github.com/yuxino/mimi/blob/5f6612c59f95e9ad12928121579958d470742952/README.md)。

---

## Wayland 原生兼容性：待专项验收

本页 Linux GUI 图与已过步骤是 **X11**；本轮没有新包原生 Wayland 图，不算 Wayland 已通过。v1.5.3 仅有项目记录的历史 Weston 前端/命令验证，不能替代本次 GNOME/KDE 全屏、焦点与多屏验收。

| 能力 | 当前核查结论 |
| --- | --- |
| 普通字幕显示/翻译 | 没有因Wayland禁用；精确新包原生待验 |
| 定位、置顶、全工作区、覆盖其他应用全屏 | 当前GTK3普通窗口路径不能保证；其Wayland置顶/stick为空实现，不能把API成功返回当实际置顶 |
| 鼠标穿透与沉浸 | Wayland input-region协议支持，应用已有路径；上游Tao留1×1输入区，完整穿透和恢复需专项验 |
| 快捷键 | 已有系统命令绑定退路；Portal自动配置是后续最小方案，尚未实现/授权绑定 |
| 控制面板、拖动、尺寸、多屏 | native拖动有路径；独立面板与自绘resize依赖全局坐标，有应用适配缺口，原生待复现 |

下一步先验当前新包实际Wayland后端，保住设置/停止/退出沉浸入口；再分小补丁处理空输入区、原生resize/parent-relative控制及Portal。KDE/wlroots可评估layer-shell，GNOME不能承诺同一协议通用。没有关闭Wayland安全限制或操作用户权限；尚未提交生产Wayland补丁，不宣称上述已修。

依据：[固定GTK3源码](https://raw.githubusercontent.com/GNOME/gtk/3.24.33/gdk/wayland/gdkwindow-wayland.c)、[锁定版本Tao路径](https://raw.githubusercontent.com/tauri-apps/tao/tao-v0.35.3/src/platform_impl/linux/event_loop.rs)、[Portal协议](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html)、[layer-shell桌面支持](https://github.com/wmww/gtk-layer-shell)。

---

## 在 Dock 中显示（独立提交，Mac 原生待验）

通用设置新增一个开关，运行时可显示或隐藏 Mimi 的 Dock 图标。默认关闭，保持今天之前的菜单栏运行方式；设置会保存，开关失败会提示并回退。隐藏后菜单栏的设置和退出入口仍保留，开启时点击 Dock 可回到设置。

**Dock 原始提交：** `01b298a74f14984658dca8ecfa66195fd89e959f`（最新 head 见顶部），独立Dock提交基于已全绿861f159。[跨平台CI36827691788](https://github.com/yuxino/mimi/actions/runs/36827691788)和[Android CI36827691667](https://github.com/yuxino/mimi/actions/runs/36827691667)均completed/success；包含Mac/Linux/Windows Rust、Windows ARM、前端和MSRV适用检查，打包/发布步骤按当前CI规则跳过。前端239测试/34文件、typecheck/build通过，lint0错误/1既有警告；测试覆盖旧配置默认、存盘重读/跨服务保留、runtime失败和写盘失败回退、非Mac隐藏及保存中防重复。

**图片：实际Mac Chromium组件 + 合成内存配置，中文/浅色，1280×900逻辑 / 2560×1800原图。不是实际Dock截图，也没有安装新包。** Windows/Linux界面不显示此项，后端拒绝Dock草稿；没有改系统Dock固定项目、正式包、签名、钥匙串或权限。此前 ad51bcc Linux引导修复包不含本提交；最新 db44d57 候选已含。

| 修改前：861f159 | 新增：01b298a，默认关闭 |
| --- | --- |
| [![修改前通用设置](https://raw.githubusercontent.com/yuxino/mimi/2fe9c32379643202e27703b0ff2f4c2a72079585/before-general.png)](https://raw.githubusercontent.com/yuxino/mimi/2fe9c32379643202e27703b0ff2f4c2a72079585/before-general.png) | [![新增Dock开关默认关闭](https://raw.githubusercontent.com/yuxino/mimi/2fe9c32379643202e27703b0ff2f4c2a72079585/after-default-off.png)](https://raw.githubusercontent.com/yuxino/mimi/2fe9c32379643202e27703b0ff2f4c2a72079585/after-default-off.png) |

[![浏览器合成配置选择开启](https://raw.githubusercontent.com/yuxino/mimi/2fe9c32379643202e27703b0ff2f4c2a72079585/after-choice-on.png)](https://raw.githubusercontent.com/yuxino/mimi/2fe9c32379643202e27703b0ff2f4c2a72079585/after-choice-on.png)

已实际操作：默认关闭→开启→切页返回保留→关闭；这里只证明浏览器状态。原生待验：Mac Dock与Cmd-Tab显示/隐藏、切回、设置及浮层焦点、菜单栏设置和退出、完整进程重启持久化、全屏浮层。需协调隔离UI-only安装窗口，不能拿CI或此图代替；当前用户的47安装包保持不动。

[固定原图、版本与SHA256](https://github.com/yuxino/mimi/blob/2fe9c32379643202e27703b0ff2f4c2a72079585/README.md)。

---

## 白底与首次引导保存修复（ad51 Linux 原生已过）

设置底色改为白色，灰色面板保持层次；深色模式颜色不变。首次引导保存配置后继续留在引导，能接着走声音和字幕步骤。

**白底版本：** #88 先前 `861f1598b34811ddde3b8ee8d83ffc5780aa4a19` 已含设置白底，跨平台与Android CI均成功。独立引导候选 `ad51bcc6d15fecaffa659140c3a6cbb7d2771f42` 另含自动引导保存修复及帮助区中性灰；尚未合入 #88/main、尚未替换本机47安装包。候选[CI 36826004273](https://github.com/yuxino/mimi/actions/runs/36826004273)全部成功；Linux单包已生成并实际下载核验；ad51 原生自动引导保存与三语流程已通过，双语问题见顶部后续修正。

以下是 **Mac Chromium 实际组件 + 合成配置，中文，1280×900逻辑 / 2560×1800原图**，不是新Linux或Mac安装包实拍。对照使用对应源码组件与同一画布/fixture；没有真实密钥或服务计费。

### 设置底色：47已安装源码对照新候选

| 修改前 | 候选修复后 |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/47-light-settings.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/47-light-settings.png) | [![候选修复后](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-light-settings.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-light-settings.png) |

### 帮助区域：去掉偏暖底色

| 修改前 | 候选修复后 |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/58-light-help.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/58-light-help.png) | [![候选修复后](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-light-help.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-light-help.png) |

### 深色模式：颜色保持

| 修改前 | 候选修复后 |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/58-dark-settings.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/58-dark-settings.png) | [![候选修复后](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-dark-settings.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-dark-settings.png) |

### 首次自动引导：保存后保持，不跳出

58a48ab Linux真实UI确认自动入口保存后关闭；浏览器同条件也复现。修复后仍停留在引导，并可继续到声音、字幕。下表为浏览器实际表单保存后对照；未用手动引导按钮绕过自动入口。

| 修改前 | 候选修复后 |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/58-auto-after-save.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/58-auto-after-save.png) | [![候选修复后](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-auto-after-save.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-auto-after-save.png) |

| 继续：声音 | 继续：字幕 |
| --- | --- |
| [![声音步骤](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-auto-continue-audio.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-auto-continue-audio.png) | [![字幕步骤](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-auto-caption.png)](https://raw.githubusercontent.com/yuxino/mimi/2b61140666f8f9a66295deb8fa0348741f6f1cc0/new-auto-caption.png) |

**新包：** [Linux 工件11145840821](https://github.com/yuxino/mimi/actions/runs/36826004273/artifacts/11145840821)，ZIP SHA256 `4eb453aaafb11fd926a113b94fefde7dcab2561c0e6e934ea17b18733a4dcc8f`；deb SHA256 `9c1ef9dd993d98263673cf86eced1f0808f5a23bf73140e7bb97add166cf8d86`。ZIP摘要与GitHub一致、CRC与包成员哈希已核对。

回归：干净 UI-only 配置启动 → 自动出现引导 → 实际服务表单填合成字段并保存 → 引导保持 → 继续到声音/字幕 → 稍后关闭/手动再开；再测异步配置加载。新增3项测试修复前2失败，修复后全部通过；前端251测试/38文件、typecheck/build通过，lint0错误/1既有警告。ad51 Linux原生已重复自动路径并通过；640×136双语浮层确认原文藏于上方，已由顶部新候选修复，须新包复验。真实服务认证/采音权限、七态、系统减少动态效果未覆盖。

[全部15张原图、computedStyle、准确版本与SHA256](https://github.com/yuxino/mimi/blob/2b61140666f8f9a66295deb8fa0348741f6f1cc0/README.md)。下面58原图是上一候选的历史对照，不能作为本次修复后的原生验收结果。

---

## 设置精简与本轮验收

集成源码 **083bb30d74a93f99d185a181f16629bdb3c8489d**；独立连续引导候选 **58a48aba8c7fcd56338fc40167d691f3ad58cd3a**。下面是新源码的安全合成截图，未覆盖当前本地安装的47ae4c5，未合并main或发版。

### 检查连接：只在需要时看详情

去掉常驻的测试原理与重复认证说明，检查结果就近显示，排障信息收进「连接详情」，按Mac/Windows/Linux分别给对应提示。安全存储与脱敏逻辑保留；服务器可达不会被说成授权成功。

同条件 before/after：实际Mac Chromium浏览器组件、浅色1280×900逻辑/2×高清原图、两条合成配置。前版47ae4c5；新版58a48ab（设置本身与#88的083bb30一致，新候选额外有页头引导入口）。均模拟「凭据不可读、服务器可达」，没有读取钥匙串、输入真实密钥或请求服务；错误图不是认证恢复证明。

| 前版47ae4c5 | 新版58a48ab：中文 |
| --- | --- |
| [![前版中文连接检查](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/before-zh-connection.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/before-zh-connection.png) | [![新版中文连接检查](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/after-zh-connection.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/after-zh-connection.png) |

### 配置名称与删除：分开摆放

名称展开区仅保留改名，删除单独放在底部；确认与会话保护仍在。引导入口移到设置页头，避免挤在删除旁；此入口位置目前只在独立引导候选。

| 前版：名称与删除混在一起 | 新版：名称和删除分开 |
| --- | --- |
| [![前版中文名称删除](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/before-zh-name-delete.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/before-zh-name-delete.png) | [![新版中文名称删除](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/after-zh-name-delete.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/after-zh-name-delete.png) |

### 英文与日文同步

同一合成错误、同一窗口条件，已检查实际正文与布局。

| English before | English after |
| --- | --- |
| [![English before](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/before-en-connection.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/before-en-connection.png) | [![English after](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/after-en-connection.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/after-en-connection.png) |

| 日本語 before | 日本語 after |
| --- | --- |
| [![日本語 before](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/before-ja-connection.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/before-ja-connection.png) | [![日本語 after](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/after-ja-connection.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/after-ja-connection.png) |

回归：翻译服务 → 编辑合成配置 → 检查连接 → 展开/折叠详情 → 改名区 → 页头使用引导打开/稍后关闭。浏览器三语、无横向溢出、引导入口实际打开关闭通过；候选完整前端248测试/37文件、typecheck/build、lint0错误/1既有warning通过。新包原生、真实凭据恢复仍待验。

### 日常CI与验收包

日常仍做前端检查和受影响平台编译/测试；共享Rust或工作流改动保守检查全部原生平台。日常不自动打所有安装包。需要原生验收时手动选Linux/Windows/macOS；保留手动full检查，正式release保持完整打包。没有增加token权限或修改分支保护。

083bb30：[集成CI](https://github.com/yuxino/mimi/actions/runs/36822039560)、[Android CI](https://github.com/yuxino/mimi/actions/runs/36822039204)，本轮均 completed/success。58a48ab：[独立候选CI及单个Linux验收包](https://github.com/yuxino/mimi/actions/runs/36822222741)，本轮 completed/success；只有 Linux 包被请求并生成，其余平台没有打包。CI 安装/启动 smoke通过，后续实际原生操作验收仍待进行，不能用浏览器图代替。

[Linux验收artifact 11144034241](https://github.com/yuxino/mimi/actions/runs/36822222741/artifacts/11144034241)，ZIP SHA256 `88914619cc236cbee401ee9769006ba291de573adaaec6c1418342c2731dff0e` 已实际下载核对上传摘要及ZIP CRC；准确源码58a48ab。

### 贡献者墙

独立文档提交083bb30：中英README同步感谢4位已核对的公开贡献者，头像链接本人GitHub；保留已有Android归属，PR67/89待合并来源单列。

[![贡献者墙实际浏览器文档预览](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/contributors-wall.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/settings/contributors-wall.png)

[完整高清原图、版本、哈希与复现边界](https://github.com/yuxino/mimi/blob/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/README.md)。用户提供的私有截图未公开。


## 连续引导新版候选 · 白灰界面

**这是新候选，未覆盖你当前正在用的本地包。** 独立源码 `58a48aba8c7fcd56338fc40167d691f3ad58cd3a`；#88 已加入呼吸灯命名与本轮设置精简，桌面 #85 仍等待视觉和新包原生验收。本地普通模式仍是已安装的 `47ae4c5`，不会自动出现合成字幕；下方截图来自隔离浏览器的合成空配置。

改动：教程内直接填真实服务表单、保存后继续检查声音和权限、点击开始字幕，再用按钮练习沉浸与退出；失败留在原处可改。白灰底，小人物点缀。快捷键只显示当前注册成功的键位；完成仍需要真正显示本次字幕。

同条件 before/after：实际 Mac Chromium 组件，中文浅色、1280×900 逻辑 / 2× 2560×1800 原图。左边前版源码 `47ae4c5`，右边新候选 `58a48ab`；均为合成配置、非 native 安装包实拍。点击任一图可看原图。新第一屏空配置，第二/三屏拍前已用同一表单保存无效合成 key 到浏览器内存；没有真实服务请求。

### 连接服务

把配置直接放在教程里，保存和出错都不再跳出去。

| 前版 47ae4c5 | 新候选 58a48ab |
| --- | --- |
| [![前版 浏览器 中文 第1屏](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-1.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-1.png) | [![新候选 浏览器 中文 第1屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-step-1.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-step-1.png) |

### 声音与权限

增加系统设置和重新检查入口，在这里点击开始字幕。

| 前版 47ae4c5 | 新候选 58a48ab |
| --- | --- |
| [![前版 浏览器 中文 第2屏](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-2.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-2.png) | [![新候选 浏览器 中文 第2屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-step-2.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-step-2.png) |

### 字幕与沉浸

可以进入、退出沉浸模式；未注册的快捷键不冒充可用。

| 前版 47ae4c5 | 新候选 58a48ab |
| --- | --- |
| [![前版 浏览器 中文 第3屏](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-3.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-3.png) | [![新候选 浏览器 中文 第3屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-step-3.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-step-3.png) |

### 英文与日文

同一候选、同平台和窗口条件。

| English | 日本語 |
| --- | --- |
| [![候选 英文 第1屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/en-step-1.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/en-step-1.png) | [![候选 日文 第1屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/ja-step-1.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/ja-step-1.png) |
| [![候选 英文 第2屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/en-step-2.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/en-step-2.png) | [![候选 日文 第2屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/ja-step-2.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/ja-step-2.png) |
| [![候选 英文 第3屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/en-step-3.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/en-step-3.png) | [![候选 日文 第3屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/ja-step-3.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/ja-step-3.png) |

### 小窗口

430×620 逻辑 / 2× 原图；内容滚动后可以点到保存，底部按钮保持可见。

[![候选 小窗口 第1屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-small-step-1.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-small-step-1.png)

[![候选 小窗口 第2屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-small-step-2.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-small-step-2.png)

[![候选 小窗口 第3屏](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-small-step-3.png)](https://raw.githubusercontent.com/yuxino/mimi/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/guide/zh-small-step-3.png)

验收步骤：打开初次使用 → 输入合成配置并保存 → 前后切步骤检查草稿/状态 → 打开声音页（浏览器只提示去应用，不打开系统权限） → 字幕页点击沉浸/退出（浏览器不会修改原生偏好） → Tab/Shift+Tab 循环 → 稍后关闭。实际内存表单保存、跨步骤草稿、8 服务选项、三语布局、正反向焦点、小窗口与关闭已通过；真实权限、真实首字幕、新候选原生包和 Wayland 快捷键实机仍待验。

当前候选前端 248 tests / 37 files、lint 0 errors / 1 既有 warning、typecheck/build 通过；前版0dfb118 Rust 536 passed / 1 ignored、fmt/strict Clippy通过；新候选远程Rust检查以准确head为准。 [准确候选远程 CI](https://github.com/yuxino/mimi/actions/runs/36822222741) 已 completed/success。[完整版本/哈希/复现边界](https://github.com/yuxino/mimi/blob/44f5c71de392fe2dcdb4e7cc7ca427e5dad4a379/README.md)。

## 最新整合状态与新增验收

当前集成head：`083bb30d74a93f99d185a181f16629bdb3c8489d`。包含呼吸灯命名、设置精简、CI优化与独立贡献者文档；classic枚举、默认和动效不变。新连续引导仍单独在58a48ab候选中。**#89 已纳入，保留 @LLLin000 的来源提交；没有合并 main 或发版。** 默认选项跟随有声音的输出，角色优先顺序是 communication → media → console；设备绑定有防抖。修正了来源 PR 正文与实际默认路径不一致、原始设备 ID 匹配和 COM 临时内存释放。Windows 实机切换/Teams/真实字幕仍待验，贡献者在 #74 的实机反馈无 exact SHA，只记录为其自报反馈。

当前083bb30 [跨平台CI](https://github.com/yuxino/mimi/actions/runs/36822039560) 与 [Android CI](https://github.com/yuxino/mimi/actions/runs/36822039204) 均已 completed/success。前版da19375跨平台和Android均已成功；前版0dfb118独立候选CI也已成功。前版 08766d3 的 9 项适用跨平台检查与 Android 已通过；不得用其结果替代新 head。下方旧 head 的 CI 和图片是历史验收；最新包新增结果以本节准确版本为准，不表示真实音频/provider 全过。

重启保存测试使用新入口：只读写明确指定的私有临时目录内非敏感偏好；普通 UI-only 不读取真实用户配置或钥匙串。Linux 08766d3 原/A/B 完整进程退出后重启及关闭动效保存已实测通过。Mac 独立候选已安装并完成下述原生回归；#85 原稿仍 HOLD，新图示仅在本地候选中，等用户最终视觉确认。

### Mac 本地实拍

**已安装独立测试版 `/Applications/mimi-dev.app`；保留正式应用与真实配置。** 本地源码 `47ae4c51c0f15a3fd6943d633d25531e06843db0` 包含 #88/#89 与单独保留的图示引导候选；#88 生产分支仍是 `08766d3`，图示待你最终看视觉，不自动合入。人物用现有官方图，没有新姿势。

实际 macOS WKWebView 原生窗口，中文/浅色、私有临时偏好、合成内存配置；下列 3 张引导及 3 张样式原图 2104×1810（940×793 逻辑窗口含系统阴影）。**不是旧浏览器 after，也没有真实服务首字幕。**

| 连接服务：把字段说明收进展开区 | 播放声音：用操作图说明下一步 |
| --- | --- |
| [![Mac 47ae4c5 原生图示第一屏，合成配置](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/guide-native-1.png)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/guide-native-1.png) | [![Mac 47ae4c5 原生图示第二屏](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/guide-native-2.png)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/guide-native-2.png) |

字幕位置示意，等待状态只写“正在等待翻译”：

[![Mac 47ae4c5 原生第三屏，真实字幕完成仍待验](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/guide-native-3.png)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/guide-native-3.png)

原样式可恢复；下方原/A/B 三图在同一窗口、双语示例和 idle 会话条件仅切样式，状态灯动效开、字幕动效关：

[![Mac 同条件原样式](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/style-classic.png)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/style-classic.png)

| A · 音节 | B · 声带 |
| --- | --- |
| [![Mac 同条件 A 音节](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/style-a.png)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/style-a.png) | [![Mac 同条件 B 声带](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/style-b-same-window.png)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/style-b-same-window.png) |

合成字幕暂停后保留，恢复能继续（同一句、640×136 原生浮层）：

| Listening | Paused |
| --- | --- |
| [![Mac 47ae4c5 Listening 合成字幕](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/synthetic-subtitle.png)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/synthetic-subtitle.png) | [![Mac 47ae4c5 暂停保留合成字幕](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/paused-subtitle.png)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/mac/paused-subtitle.png) |

**本机实际通过**：B 样式/两个独立动效/英文语言的完整进程退出重启保留，切回中文；原/A/B 可切换及恢复原样式；空配置安全表单保存仅内存合成值；仅译文→双语、暂停保留/恢复；沉浸说明/开启/从设置退出；停止保留合成归档；原生保存框取消后仍可再导出，TXT 中英合成内容/时间戳校验；搜索无匹配、清空当前记录；引导重复打开/三屏切换/稍后关闭，普通与放大窗口操作。Mac 原生 select 菜单自动化受限，DeepLX 高级切换未记为本机通过；没有确认这是产品缺陷。已有 dev WebView 非敏感存储沿用，不宣称全新浏览器存储的首次自动弹出已验。

已安装二进制 SHA256 `3b0566a94dd719a4f8013d8556e75b909b82c4aa2db43eb271877e5007d7bfa0`；`codesign --verify --deep --strict` 通过，前后指定签名要求一致：`app.yuxino.mimi.dev` + 现有证书 `C45731EE0170184A7542CBB1972420E319AC9D56`。正式 mimi.app 二进制前后校验一致。canonical 535 Rust passed/1 ignored、234 前端测试及其他完整检查通过；[本地候选 exact CI 9 项适用检查成功](https://github.com/yuxino/mimi/actions/runs/36765984084)，未发布。

### Linux 新包重启与导出

**设置退出再打开仍是你的选择，取消保存也能再导出。** 云 QA 实际原生 `08766d354be56ffedc2883da75754ec609a4beda`，英文浅色/私有临时偏好/合成字幕；1180×812 原图，原/A/B 同条件只改样式，状态灯动效关、字幕动效开。

[![Linux 08766d3 原样式完整进程重启后](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/linux/original-restarted.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/linux/original-restarted.jpg)

| A 完整退出后重启 | B 完整退出后重启 |
| --- | --- |
| [![Linux 08766d3 A 重启保留，动效关闭](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/linux/a-restarted.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/linux/a-restarted.jpg) | [![Linux 08766d3 B 重启保留，动效关闭](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/linux/b-restarted.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/linux/b-restarted.jpg) |

[![Linux 08766d3 原生合成 TXT 导出成功](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/linux/export-saved.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f1f98f8db22286e12ea69b9f96f9f303af8e1124/linux/export-saved.jpg)

通过：原/A/B 与关闭动效完整重启保留，预览/Listening 浮层同步、暂停恢复、连续切换最终选择、停止归档保留、GTK 保存框取消→重开→TXT 保存及内容。artifact `11120362032`，ZIP SHA256 `c3ea69fd26df47bb6699a9042fe7653c00af2636ff8552337d0e69a5bbfc1c44`。这组是最新 #89 包实测，不借旧 f4c208c 图片宣称通过。云 QA 与本机测试 app 均已正常退出，GUI 已释放。

[原图哈希、源码/签名/二进制与操作步骤](https://github.com/yuxino/mimi/blob/f1f98f8db22286e12ea69b9f96f9f303af8e1124/README.md)。**仍未覆盖**：全七态与系统减少动态效果的本轮原生时序、流式多句/长句、全局快捷键焦点/全部最小尺寸、持久历史删除恢复、磁盘写入失败、旧组合真实凭据迁移、Windows 真机/Teams/声卡切换、真实音频/provider/权限与第一条真实字幕；钥匙串根因仍未解，PC 候选最终审美待你确认。没有合并、发版或关闭 issue。

### 桌面三屏图示候选（本地体验，待你看视觉）

**改成三张操作图：连接服务 → 播放声音 → 看字幕位置；配置字段和费用展开再看。** 中英日同步，使用现有官方人物图，还没有新增角色姿势。准确本地候选 `47ae4c51c0f15a3fd6943d633d25531e06843db0`，独立分支 [`feat/local-morning-preview`](https://github.com/yuxino/mimi/tree/47ae4c51c0f15a3fd6943d633d25531e06843db0)。它含 #88/#89 和这套新引导，**不在 #88 生产分支里；#85 原稿仍 HOLD，没有 main 合并或发布。**

下方是实际挂载 App 的 Mac Chromium 浏览器实拍，1280×900 逻辑、2×高清原图，浅色、私有空配置。不是生成 HTML 图片，也不是 native Mac/Linux 包实拍。三语言×三屏无横向溢出，8个服务选项和稍后关闭已实际检查；第一条真实字幕、原生权限和最终审美仍待验；原生候选新图与操作结果见上方 Mac 本地实拍。

| 中文：连接服务 | 中文：播放声音 |
| --- | --- |
| [![桌面新图示候选 zh step 1，47ae4c5 浏览器空配置](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-1.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-1.png) | [![桌面新图示候选 zh step 2，47ae4c5 浏览器空配置](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-2.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-2.png) |

[![中文：字幕位置示意，尚未完成真实字幕](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-3.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/zh-step-3.png)

| English：连接服务 | English：播放声音 |
| --- | --- |
| [![桌面图示候选 en step 1](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/en-step-1.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/en-step-1.png) | [![桌面图示候选 en step 2](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/en-step-2.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/en-step-2.png) |

[![English：字幕位置候选](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/en-step-3.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/en-step-3.png)

| 日本語：连接服务 | 日本語：播放声音 |
| --- | --- |
| [![桌面图示候选 ja step 1](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/ja-step-1.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/ja-step-1.png) | [![桌面图示候选 ja step 2](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/ja-step-2.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/ja-step-2.png) |

[![日本語：字幕位置候选](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/ja-step-3.png)](https://raw.githubusercontent.com/yuxino/mimi/caebb135df6511d3777267f53effe015506e31c4/ja-step-3.png)

回归入口：候选分支运行单个轻量 Vite，打开 `/first-run-fixture.html?lang=zh`，再用 en/ja；检查三步骤、服务下拉、详情展开、填写配置、返回/稍后、重复打开及键盘焦点。浏览器入口不连服务、不访问钥匙串，也不能把示例字幕算作完成。[固定9张原图、版本、哈希与已测边界](https://github.com/yuxino/mimi/blob/caebb135df6511d3777267f53effe015506e31c4/README.md)。

本地候选 `scripts/check.sh` 已通过：535 Rust（1 ignored）、234前端、格式/严格Clippy/typecheck/build及安装恢复/签名脚本测试。原生测试包单次构建安装进行中；[候选 exact CI](https://github.com/yuxino/mimi/actions/runs/36765984084) 跟到终态后更新，不将浏览器图冒充安装包通过。

### Android 新文案原生实拍

**等待时只说“正在等待翻译”；共享失效时才提示重新开启，并保留按钮。** 实际 API35 Google APIs ARM64 Pixel7 模拟器，1080×2400、浅色、中英日；空配置和合成共享结束状态，无凭据、权限请求、MediaProjection 或付费服务。Android 源码与当前 `08766d354be56ffedc2883da75754ec609a4beda` 一致，App APK SHA256 `5fc214db00643c6a9c9a0f6b7303f319abdad8e71e6bc9da9b005b06b20235b0`。

专项真实操作：连续打开权限/等待页 → 合成共享结束 → 刷新 → 检查并点击重新开启按钮；三语言均通过，每种3张共9张。修复了旧弹窗关闭回调误清新弹窗的实际重复打开问题。模拟器已正常退出。

| 中文：等待 | 中文：共享结束与操作入口 |
| --- | --- |
| [![API35 zh waiting，新文案，08766d3](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/zh-waiting.png)](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/zh-waiting.png) | [![API35 zh sharing-ended，新文案，08766d3](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/zh-sharing-ended.png)](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/zh-sharing-ended.png) |

| English：等待 | English：共享结束与操作入口 |
| --- | --- |
| [![API35 en waiting，新文案，08766d3](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/en-waiting.png)](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/en-waiting.png) | [![API35 en sharing-ended，新文案，08766d3](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/en-sharing-ended.png)](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/en-sharing-ended.png) |

| 日本語：等待 | 日本語：共享结束与操作入口 |
| --- | --- |
| [![API35 ja waiting，新文案，08766d3](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/ja-waiting.png)](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/ja-waiting.png) | [![API35 ja sharing-ended，新文案，08766d3](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/ja-sharing-ended.png)](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/ja-sharing-ended.png) |

| 中文权限页 | English access | 日本語 権限 |
| --- | --- | --- |
| [![API35 zh 权限页，新文案](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/zh-access.png)](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/zh-access.png) | [![API35 en 权限页，新文案](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/en-access.png)](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/en-access.png) | [![API35 ja 权限页，新文案](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/ja-access.png)](https://raw.githubusercontent.com/yuxino/mimi/54bc32139450891304cb196da41dcd70af0c3cef/ja-access.png) |

回归：在独立空模拟器安装 App/test APK，运行 `adb shell am instrument -w -e guide_copy true -e locale zh -e theme light app.yuxino.mimi.android.test/app.yuxino.mimi.android.UiSmokeInstrumentation`，再换 en/ja。共享结束由 instrumentation 注入，按钮仅计数，不会真的请求授权。实际音频、系统撤权、物理 Android10/14/15、Bluetooth、真实首字幕未测。本轮9张不冒充重跑旧48张全套。[固定原图、APK/testAPK 哈希与完整边界](https://github.com/yuxino/mimi/blob/54bc32139450891304cb196da41dcd70af0c3cef/README.md)。

---

这次把今天的功能放在同一个 PR 验收：字幕按句成块，诊断反馈不挤界面，DeepLX 填错地址后方便修改，历史导出取消后仍能看字幕，Android 首次使用有步骤引导。

**DeepLX 配置交互已纳入；下述为此前检查记录。** 来源 `93442f07a277082ec603019f2f38b80b504fe1ff`，整合 head `26894aebc4264ee563fd679a001f048e118587c0`。普通阿里配置默认跟随当前服务，在高级设置选择 DeepLX 文字翻译；仅开放已有 Audio3 组合，保留旧配置和安全存储。此前 `197f165` 十项远程 CI 已通过，并完成下述云 Linux UI-only 检查；`38bc4df` 十项 CI 也已全绿并完成导出补验；A/B 选择及等待文案的 `f4c208c` 十项 CI 已全绿；最新 `26894ae` 仅补三语言权限提示，十项适用 CI 已全绿。

**A 音节 / B 声带都保留，用户自己选。** 用户已明确确认；当前分支新增可保存的“状态灯样式”，默认原样式，A/B 可选并可恢复，不改已有动效偏好。设置预览与悬浮窗使用同一选择，沿用跨窗口广播、独立动效开关和系统减少动态效果。仅表达既有会话状态，没有新增音量或音频字段。原/A/B 选择、设置预览与 Listening 浮层同步及本轮 Linux 截图已补验；重启、系统 reduce 与全七态仍待验，暂不合并。

### Linux 原生验收

**现在可以在设置里选原样式、A 音节或 B 声带，预览和悬浮窗一起切换。** 下方是云 Linux 原生实拍，不是浏览器预览：准确安装包 `f4c208cdf7aa752a26ccee949386a72e49c8e5d1`，英文/浅色、隔离 UI-only 合成配置。设置原图 1180×812，浮层 640×136；没有真实音频或服务连接。此前 `26894ae` 与该包桌面代码相同；当前已另含 #89 和隔离设置验收入口，图片仍标实际 `f4c208c`，不替代最新包回归。

原样式（这张状态灯动效已关闭）：

[![Linux f4c208c：原样式设置预览，动效关闭](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/original-preview.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/original-preview.jpg)

| A 音节：设置预览 | B 声带：设置预览 |
| --- | --- |
| [![Linux f4c208c：A 音节设置预览](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/a-preview.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/a-preview.jpg) | [![Linux f4c208c：B 声带设置预览](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/b-preview.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/b-preview.jpg) |

两张设置图来自不同操作阶段：A 会话尚未启动，B 会话暂停；都选中对应样式并开启动效，不冒充同一会话状态的精确前后对照。

| A 音节：Listening 浮层 | B 声带：Listening 浮层 |
| --- | --- |
| [![Linux f4c208c：A 音节 Listening 原生浮层](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/a-overlay.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/a-overlay.jpg) | [![Linux f4c208c：B 声带 Listening 原生浮层](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/b-overlay.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/b-overlay.jpg) |

**云 QA 本轮实际通过**：三样式选择及设置预览/Listening 浮层同步、暂停恢复、关闭动效后保留静态形状、同进程切页保留、诊断说明移入可选详情。动态行为根据连续操作观察记录，静图不单独证明动效时序。**仍未测**：重启持久化、系统减少动态效果、全七态时间序列、真实音频/provider、Android本轮新界面。下面的 Mac 浏览器视频保留作设计对照，不能替代这些未测项。[原图、哈希和回归步骤](https://github.com/yuxino/mimi/blob/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/README.md)。

### A 音节与 B 声带

[![原样式 / A 音节 / B 声带，同条件高清对照](https://raw.githubusercontent.com/yuxino/mimi/bc7be89927f21540703db91d7172bab41a612d6d/sound-directions-hd.png)](https://raw.githubusercontent.com/yuxino/mimi/bc7be89927f21540703db91d7172bab41a612d6d/sound-directions-hd.png)

https://github.com/user-attachments/assets/5de961d8-4c6d-4ad5-8635-f76d1702b19c

上图/视频是实际 Mac 浏览器、合成字幕和七状态：原灯为精确 `197f165`，A/B 来源 `d72d5be`，同 Timeline/字体/画布；**不是新设置页、不是 Linux 安装包截图或音量响应。** 用户后来选择两种都保留，图中“等待反馈”是拍摄时的旧状态。22.042秒、1440×1080，977原始CDP样本，原时序/静止间隙保留，无插帧、改速或固定60fps宣称。[公开原图/视频、哈希及边界](https://github.com/yuxino/mimi/blob/bc7be89927f21540703db91d7172bab41a612d6d/README.md)。

**前版脉冲候选已纳入集成分支。** 独立来源 `3fb46654` 已移植为 `259bce1`，没有合入旧预览分支历史；#67 字幕文字和句块不变。下方直接展示新图与实际视频，属于 Mac 浏览器合成状态对照，上方 Linux 已补验当前 A/B 选择与 Listening 同步；七态原生动效时序仍待验，不表示 #88 最终验收或发布。

**#67 已纳入，保留贡献者的句块、文字渐入、定稿稳定和年龄淡出。** 细修只涉及分隔线、时间显示和状态小动效，仍需看图及同包体验确认；不扩为整页重设计。PC 引导 #85 按用户反馈继续 HOLD。

Refs #67, #72, #73, #76, #84, #86, #89；issues #70, #74, #75, #77, #78, #82, #83, #87。

#72/#73/#76 在用户改为“集成 PR”之前已分别合入 main，当前基线为 `d190877`；此 PR 补入 #67/#84/#86 和验收记录，来源 PR 保留。**此前完整 head：`26894aebc4264ee563fd679a001f048e118587c0`。本轮 Linux 图明确标 `f4c208c` 原生实拍；其他图复用来源 PR 或预览，各自标拍摄版本，均不冒称当前 head 实拍。** 点击图片可看公开原图。

## 1. 字幕按句阅读，保留原设计（#67 / #87）

把同一句的原文和译文放在一起，长句换行仍属于同一个句块，避免读着读着串到另一句。原有流式文字渐入、定稿不重播入场、老句淡出、行数预算及两项动效选择都保留。

**前版对照：状态切换不重新跳一下，暂停慢慢收住、恢复接着动。** 保留18/40px范围和既有七状态；等待为慢弧、聆听为小 halo、识别为轻 ripple、翻译为反向双弧。这里的小灯只表达会话状态，不测音量或识别进度。

实际 Mac Chromium 浏览器、同画布/主题/尺寸/合成字幕。上行是 #88 精确指示器基线 `42f7583`，下行是候选 `3fb4665`；两行共享同一 baseline Timeline、字体与样式。DeepLX head `fcdc1a9` 的这些文件也与参考相同。**不是 Linux 安装包，也不是原生整窗或真实音频。**

[![新连续状态动效：修改前与候选同条件高清对照](https://raw.githubusercontent.com/yuxino/mimi/e7a1a38a6a5b9c956dcd8bac987049134a5d9191/phase-continuity-hd.png)](https://raw.githubusercontent.com/yuxino/mimi/e7a1a38a6a5b9c956dcd8bac987049134a5d9191/phase-continuity-hd.png)

前版实际录制视频（20.445 秒，原时序；声音感 A/B 已在上方展示）：

https://github.com/user-attachments/assets/300653f4-89b9-4f9a-bf20-9cc05a8184db

视频是 1440×1032 H.264，1016 原始 CDP 帧；变动画面跨度17.556秒，平均57.814fps，静止间隙和末尾2.887秒保留。无插帧、改速或固定60fps宣称。用户已观看确认可以公开贴入验收；录制方实际检查识别、暂停、恢复帧。[公开原图/原MP4、SHA256、平台及回归边界](https://github.com/yuxino/mimi/blob/e7a1a38a6a5b9c956dcd8bac987049134a5d9191/README.md)。

独立变更 `259bce1` 仅涉及 PulseRing、专用 CSS 和证据 note。七个动画保持原实例，活跃状态间480ms过渡；非活跃状态520ms收住后暂停时钟，恢复续时；关动效立即静止。#67 句块、文字渐入、定稿稳定、年龄淡出、两个独立开关均保留。旧统一脉冲在更早 `2795365` 已引入，不归责于 #67 贡献者。

来源检查：typecheck、组件 eslint、diffcheck 通过；浏览器七实例在识别/翻译之间保持身份，暂停 ready 后450ms时钟不变，恢复同实例续时；显式关闭和系统reduce均running0。**新 head 完整 CI 另列下方，Linux 安装包 exact-head UI-only/原生窗口仍待验。当前用户已授权独立本机 dev 候选安装；真实凭据/付费/权限交互仍需用户在场。**

回归：原文/译文/双语、长句/流式增长/定稿/老句淡出/窄窗；沉浸模式无句线；分别关闭两个动效及系统减少动态效果、显式覆盖；七状态连续切换、暂停/恢复、40/18px；随后协调固定包，在真实 Linux UI-only 核对同序列并记录 exact commit/artifact hash 和原生截图。

另保留 `6ffdbff` 的同文历史句回归、#72短标签与音源几何。高级 DeepLX 与 Android 设置未被这个动画补丁覆盖。[原贡献者 Windows 运行截图](https://github.com/yuxino/mimi/pull/67#issuecomment-5888110417)。

<details>
<summary>此前三行本地预览（历史材料，不当本次新候选证据）</summary>

[![此前原Timeline／本地baseline／旧细修对照](https://raw.githubusercontent.com/yuxino/mimi/6d4206648eab09c7b0523dba0ce8b5fc8194a407/comparison-hd.png)](https://raw.githubusercontent.com/yuxino/mimi/6d4206648eab09c7b0523dba0ce8b5fc8194a407/comparison-hd.png)

三行分别使用 #67 Timeline `3128c46`、本地 `a7b6297`、其细修 `113a340`，支持模块/画布来自本地 baseline，不是精确 main/#88 原生前后图。[原始边界](https://github.com/yuxino/mimi/blob/6d4206648eab09c7b0523dba0ce8b5fc8194a407/README.md)。

</details>

## 2. 复制诊断后提示浮在上面，侧栏不跳（#72 / #75）

原先成功提示占一行；现在是会自动消失的 toast，复制诊断时不用追着移动的页面读。

实际 macOS 1.5.5 原生 UI-only，中文/浅色，760×720 逻辑尺寸、2×原图，同“通用”页面、合成状态。前图是此前迭代 `2ec1b92`，后图是 #72 最终 toast 构建；源码证据固定在 `4a8e550`，后二进制 SHA256 `96fab3738fc995916a44ec506502047060c1e27d676488b6aafda8ba64f3fb36`。

| 修改前 | 修改后 |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/4a8e5502a06f66a1d04bd78310ff8afd35731f32/docs/qa/assets/windows-audio-source/diagnostics-before-inline-2ec1b92.png)](https://raw.githubusercontent.com/yuxino/mimi/4a8e5502a06f66a1d04bd78310ff8afd35731f32/docs/qa/assets/windows-audio-source/diagnostics-before-inline-2ec1b92.png) | [![修改后](https://raw.githubusercontent.com/yuxino/mimi/4a8e5502a06f66a1d04bd78310ff8afd35731f32/docs/qa/assets/windows-audio-source/diagnostics-after-toast.png)](https://raw.githubusercontent.com/yuxino/mimi/4a8e5502a06f66a1d04bd78310ff8afd35731f32/docs/qa/assets/windows-audio-source/diagnostics-after-toast.png) |

回归：复制诊断，等 3 秒确认消失；到期前再复制应延长同一条提示；关闭 ×、切页、关闭/重开检查残留；核对复制内容没有 key、字幕、设备名或私人地址。剪贴板拒绝需受控注入，不拿它证明真实服务错误。音源选择/采音状态也随 #72 纳入，但**Windows Teams、耳机和热插拔实机未验**。[详细步骤](https://github.com/yuxino/mimi/blob/4a8e5502a06f66a1d04bd78310ff8afd35731f32/docs/qa/windows-audio-source.md)。

**补充外部实机反馈（2026-09-30，贡献者 LLLin000 自报）**：在 Windows 11 的上游 #72，运行时由 Realtek 切到 ToDesk Virtual Audio，来源行随默认设备切换并显示“收到声音”，未出现重连或报错；停止音频源后显示“暂未采到声音数据”。[原始实测评论](https://github.com/yuxino/mimi/issues/74#issuecomment-5912332669)。

这条反馈未提供 exact SHA，是贡献者自报，不是整合者独立复测，也不是 #88 同包验收。它补充了默认输出切换和两个状态的实机观察，尚不能证明完整声音三态、真实字幕效果、Teams/蓝牙或拔插恢复。#74/#78 仍保持开放。评论中的默认设备角色、静音计费、重插恢复及单应用采集建议待代码和官方资料核查，不扩入本次 #88。

### 诊断说明收进详情

**按钮下面保持简洁，展开“查看详情”再看脱敏与公开反馈提醒。** 以下是同一 Linux `f4c208c` 原生包、英文浅色、合成诊断，不读取用户凭据。常驻区可在上方原/A/B设置图查看；本图是展开后的状态。

[![Linux f4c208c：诊断说明仅在展开详情后出现](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/diagnostic-details.jpg)](https://raw.githubusercontent.com/yuxino/mimi/f612b51a1aac2f3b1f0b20f88691f15a19a1bc29/diagnostic-details.jpg)

回归已确认常驻说明移入详情；实际脱敏保留。新包复制/剪贴板拒绝等路径须按各自证据验收，不能由这张图自动视为通过。

## 3. 普通配置填 Key，高级设置再选 DeepLX（#73 / #70）

普通阿里配置仍只需填一个 Key；「高级设置 → 文字翻译」默认跟随当前服务。选择 DeepLX 后明确显示「阿里负责识别，DeepLX 负责翻译」，复用原识别 Key；其他服务不开放尚未实现的组合，DeepLX 移出新增语音服务列表。旧 profile ID/provider/安全存储账户不变，地址和 token 只写入安全存储，切回跟随服务保留目的地配置。

来源实现 `482da0f`、最终交接 `93442f0` 已纳入。整合者另恢复旧迁移/凭据回归仍在调用的测试辅助方法（仅 `cfg(test)`），避免隔离探针遗漏完整应用测试编译。[交接与测试边界](https://github.com/yuxino/mimi/pull/88#issuecomment-5912410350)。

**云 Linux 原生 UI-only 已在 `197f165` 验证**：高级默认 Follow、复用内存 ASR Key、错误地址保留/就近错误/聚焦、localhost 合成有效地址＋空 token 保存、切回 Follow。未调用真实服务。匹配 before/after 公开图仍待补；下面旧图只证明此前地址错误反馈，不是新布局截图。

提交错误地址后保留输入，并在字段旁显示错误、聚焦问题位置，避免重新填整张表。

实际 Linux AppImage，英文，1180×812；同一个已配置的 DeepLX「Update Credentials」表单，合成 ASR `qa`、无效地址、token 留空。前 `367013c`，后 `8437b9b`；素材固定于证据提交 `e98f352`。

| 修改前 | 修改后 |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/e98f352e8e7699695ec8ba5d12bc7880310e591e/docs/qa/pr73/assets/before-invalid-endpoint.jpg)](https://raw.githubusercontent.com/yuxino/mimi/e98f352e8e7699695ec8ba5d12bc7880310e591e/docs/qa/pr73/assets/before-invalid-endpoint.jpg) | [![修改后](https://raw.githubusercontent.com/yuxino/mimi/e98f352e8e7699695ec8ba5d12bc7880310e591e/docs/qa/pr73/assets/after-invalid-endpoint.jpg)](https://raw.githubusercontent.com/yuxino/mimi/e98f352e8e7699695ec8ba5d12bc7880310e591e/docs/qa/pr73/assets/after-invalid-endpoint.jpg) |

回归：普通首次配置只填 Key；展开高级设置选择 DeepLX，确认不需重复识别 Key且链路说明清楚；新地址填 `bad` 后输入保留、字段旁错误可见且聚焦；检查旧 DeepLX profile、切回跟随服务、其他服务无 DeepLX 选项、合成存储写入/删除回滚。**没有验证私有 DeepLX 服务认证或真实字幕。** [来源验证与复现](https://github.com/yuxino/mimi/blob/e98f352e8e7699695ec8ba5d12bc7880310e591e/docs/qa/pr73/README.md)。

## 4. Android 可以找到采音诊断，未知状态不当成成功（#76 / #78）

权限、采音和字幕状态分别观察，方便判断卡在哪一步；仅有心跳或示例文字不会显示成采音成功。

实际 API 35 / Android 15 Pixel 7 模拟器，1080×2400、density 420、中文/浅色、同一个空配置首页。前 `5f2a595`，后 `2471629`，两版独立构建后在同一模拟器清空测试数据拍摄；只比较闲置首页诊断入口。

| 修改前 | 修改后 |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-capture-health/before.png)](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-capture-health/before.png) | [![修改后](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-capture-health/after.png)](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-capture-health/after.png) |

回归：空配置启动，检查诊断入口；分别注入无近期 PCM、零 PCM、非零合成 PCM，不能混为一类，也不能声称已有真实字幕。**这些图不证明真实系统采音、物理 Android 10/14/15 或蓝牙路线。** [APK 哈希、步骤与未测范围](https://github.com/yuxino/mimi/blob/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/README.md)。

## 5. 导出取消后字幕仍在，删历史也不会误跳（#84 / #77）

取消 TXT 导出后保留当前字幕；重复点击删除只发一次，删除 A 尚未结束时切到 B，完成后仍留在 B。

真实 SessionExport React 组件＋模拟 IPC，macOS 上 Chromium 152，英文/浅色，1000×1100、UTC；同一组合成 current/A/B 字幕。前 `5f2a595`，后实现 `01b01af`，后续证据提交 `42e2553` 未改实现。**不是原生保存对话框。**

取消导出：

| 修改前 | 修改后 |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/42e2553990fd85f14cd74058818725a0c607d789/docs/qa/history-interaction/before-export-cancel.png)](https://raw.githubusercontent.com/yuxino/mimi/42e2553990fd85f14cd74058818725a0c607d789/docs/qa/history-interaction/before-export-cancel.png) | [![修改后](https://raw.githubusercontent.com/yuxino/mimi/42e2553990fd85f14cd74058818725a0c607d789/docs/qa/history-interaction/after-export-cancel.png)](https://raw.githubusercontent.com/yuxino/mimi/42e2553990fd85f14cd74058818725a0c607d789/docs/qa/history-interaction/after-export-cancel.png) |

A 删除延迟、期间选择 B：

| 修改前 | 修改后 |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/42e2553990fd85f14cd74058818725a0c607d789/docs/qa/history-interaction/before-delete-completion.png)](https://raw.githubusercontent.com/yuxino/mimi/42e2553990fd85f14cd74058818725a0c607d789/docs/qa/history-interaction/before-delete-completion.png) | [![修改后](https://raw.githubusercontent.com/yuxino/mimi/42e2553990fd85f14cd74058818725a0c607d789/docs/qa/history-interaction/after-delete-completion.png)](https://raw.githubusercontent.com/yuxino/mimi/42e2553990fd85f14cd74058818725a0c607d789/docs/qa/history-interaction/after-delete-completion.png) |

回归：打开合成字幕，导出后取消并等几次刷新，字幕/选择/搜索/页码应保留；延迟删除 A、连续点确认、选 B 后完成 A，确认只发一次且 B 保留。成功保存、失败重试和原生文件对话框待统一包验；历史删除只能用隔离合成目录。[16 项组件用例与原生待验步骤](https://github.com/yuxino/mimi/blob/42e2553990fd85f14cd74058818725a0c607d789/docs/qa/history-interaction.md)。

## 6. Android 首次打开有可跳过的引导（#86 / #82）

### Android 引导画面

**以下5张保留为历史基线；当前文案的9张新原生图已在正文上方直接展示，不把旧图当新 after。** 当前分支已将等待改为“正在等待翻译”，删除常驻验收规则，权限步骤只给下一步操作；仅共享失效时显示“屏幕共享已结束，请重新开启”并可直接重新开启。中英日同步，真实首字幕判定仍留逻辑中。

把服务、权限、音频和字幕分成可操作步骤，退出后还可以继续；示例字幕不算完成真实首字幕。

实际 API 35 Pixel 7，1080×2400，中文/浅色、空配置、同条件首次打开。前 #76 `2471629`，后 UI `b847050`；最终 `1e7de49` 只改 xAI 价格链接，画面相同。

| 初始基线 | 前版首次引导（历史画面） |
| --- | --- |
| [![修改前](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/before.png)](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/before.png) | [![修改后](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/zh-light/guide-first-light.png)](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/zh-light/guide-first-light.png) |


**继续看实际步骤：权限未允许、声音尚未开始、字幕仍未完成。** 三张均为上述同一 UI SHA 的真实 API35 Pixel7 模拟器中文浅色图，合成配置；没有真实 provider/采音或首字幕成功。角色插画未装入当前 Android 稿。

| 权限未允许 | 声音尚未开始 |
| --- | --- |
| [![Android 引导：权限未允许](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/zh-light/guide-permissions-denied-light.png)](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/zh-light/guide-permissions-denied-light.png) | [![Android 引导：声音尚未开始](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/zh-light/guide-audio-not-started-light.png)](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/zh-light/guide-audio-not-started-light.png) |

[![Android 引导：字幕尚未完成](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/zh-light/guide-caption-not-complete-light.png)](https://raw.githubusercontent.com/yuxino/mimi/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/android-first-run/zh-light/guide-caption-not-complete-light.png)

回归：首次启动进入引导，跳过后恢复；检查缺失/合成配置、权限拒绝与撤销、MediaProjection 取消、无采音和未完成字幕状态；再检查升级安装不强弹。已有中文/日文浅色、英文深色共 48 张真实模拟器图，服务未连接，真实首字幕仍待验。**PC #85 没纳入，也未因 Android 图自动视为视觉通过。** [完整图册、命令和 SHA](https://github.com/yuxino/mimi/blob/f1ca9db23532d4c5ec5c8a1ee5b0eb0242791a0e/README.md)。


## 桌面引导待改

**#85 HOLD，未纳入 #88；下图是前版公开稿，供同页看全，不是新插画引导。** 最新 PR85 文案小修为 `4177c48a05c46c842ff98372c37f3df64a28ed23`，本工作区独立移植、未合入集成；task12 原工作区仍 `37164df`，后续须从新 head 接续。新稿仅把等待标题改为“正在等待翻译”并删除内部规则/重复段落，中英日同步，该旧稿未实现三屏图示；正文上方已展示独立新候选，角色仍用现有官方图，没有新姿势。`4177c48` 的九项远程检查已全部通过，发布步骤跳过；[独立分支 CI](https://github.com/yuxino/mimi/actions/runs/36745211989)，[Linux 验收包](https://github.com/yuxino/mimi/actions/runs/36745211989/artifacts/11111994098)。改后截图待补；下图准确标前版。用户此前否定文字堆砌，这个问题尚未解决。三屏新候选已在正文上方并补 Mac 原生实拍；新角色姿势和用户最终视觉确认仍是缺项。

前版浏览器前后是实际 App/引导组件、1280×720、中文浅色、合成空配置，基于同一 `5f2a595`。前图未有引导，后图为当前公开的文字稿；图片固定在 `37164df`，不代表集成包或原生整窗。

| 无引导的基线 | 前版桌面文字稿（待改） |
| --- | --- |
| [![桌面浏览器基线：无引导](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/before-empty-5f2a595.png)](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/before-empty-5f2a595.png) | [![桌面现存文字引导：待改未纳入集成](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/after-empty-services.png)](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/after-empty-services.png) |

以下三张是实际 macOS WKWebView 原生、隔离空配置 UI-only，实拍行为构建 `e96aba459ecf15d2f03d4eed66cc9792ead1b6c5`；后续 `37164df` 为记录/文案与 URL 修正，素材复用并标明拍摄版本。没有真实密钥、收费会话或首字幕。**原生行为测试通过不等于用户认可此视觉。**

| 当前配置检查（合成） | 等待字幕（未完成） |
| --- | --- |
| [![桌面原生配置检查：现存稿待改](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/native-empty-check-e96.png)](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/native-empty-check-e96.png) | [![桌面原生等待字幕：现存稿待改](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/native-waiting-caption-e96.png)](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/native-waiting-caption-e96.png) |

[![桌面原生沉浸模式提示：现存稿待改](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/native-immersive-help-e96.png)](https://raw.githubusercontent.com/yuxino/mimi/37164df1f428b055d041a9e7982bdc43cce25af2/docs/evidence/first-run/native-immersive-help-e96.png)

以上桌面 5 张、Android 5 张都直接内嵌原图，可点击放大。[桌面现存源码与原生证据边界](https://github.com/yuxino/mimi/blob/37164df1f428b055d041a9e7982bdc43cce25af2/docs/plans/2026-09-30-first-run-guide.md)。

## 检查与待验

- **此前 `26894aebc4264ee563fd679a001f048e118587c0` 十项适用 CI 已全部通过**：[桌面/跨平台](https://github.com/yuxino/mimi/actions/runs/36746394981)、[Android](https://github.com/yuxino/mimi/actions/runs/36746394586)。[当前 Android debug 验收 APK](https://github.com/yuxino/mimi/actions/runs/36746394586/artifacts/11112134770)，归档 SHA256 `367bf9b5891a2eee5b0c47002c80df50182223f6a0c08f5d42fae8ac781bad03`。最后三行只把 Android 权限提示从完成判定说明改为下一步操作，中英日同步，XML 解析与 diffcheck 通过。前一 head `f4c208cdf7aa752a26ccee949386a72e49c8e5d1` 的十项 CI 已全部通过：[桌面/跨平台](https://github.com/yuxino/mimi/actions/runs/36744844840)、[Android](https://github.com/yuxino/mimi/actions/runs/36744844532)。本机轻量 typecheck/组件 eslint、19 项定向前端测试、fmt/diffcheck 通过，没有 Rust/native 编译、安装或启动。首轮 `8fc0fa6` 后端因 Serde 回退注解位置编译失败，`f4c208c` 修正并通过，失败版本不能当可用包。
- **可用于本轮桌面回归的准确 Linux 包**：[`f4c208c` artifact 11111973243](https://github.com/yuxino/mimi/actions/runs/36744844840/artifacts/11111973243)，ZIP SHA256 `48b0da22254a65f9fc8939e563d87d0a6d318eaf39cf0fa96dae5fe2ee916fb6`。`26894ae` 相比它只有三份 Android XML 文案变更，桌面代码完全相同；如复用须仍标实际包为 `f4c208c`，不能冒称 `26894ae` 实拍。状态灯选择与预览/Listening 浮层同步、暂停恢复、静态形状、切页保留及简化诊断区已补验；重启/系统 reduce/全七态仍待测。
- **云 QA 已补验 `38bc4df` 的实际 Linux 导出**：核验ZIP哈希；停止保留合成归档、真实原生对话框取消后保留并再导出、TXT中英/时间戳保存、把普通文件误当目录时原生报错、修正路径重试均通过，两次文件一致。后端磁盘写入失败未覆盖。隔离应用已正常退出，不能把路径选择报错当后端写盘故障通过。旧版本结果不替代本轮选择/文案的同包验收。

- **此前 `38bc4dfb065827ef8f1f8b365029343b43d719a0` 新增最小 UI-only 导出入口，十项适用 CI 全绿**：[桌面/跨平台](https://github.com/yuxino/mimi/actions/runs/36734613924)、[Android](https://github.com/yuxino/mimi/actions/runs/36734613175)。`MIMI_UI_TEST=1` 加 `MIMI_UI_TEST_EXPORT=1` 可在停止后保留有界、纯合成的内存归档；生产 finalization 不改，不访问真实历史/凭据，也不新增采音或联网。不开新增开关时仍清空。新启动、清空、关闭保留仍会重置。[隔离入口和回归步骤](https://github.com/yuxino/mimi/blob/38bc4dfb065827ef8f1f8b365029343b43d719a0/docs/plans/2026-09-30-ui-test-export-fixture.md)。该版本已完成上述云QA取消/保存/路径重试，后端磁盘写入失败仍未覆盖。
- **38bc4df 新 Linux 包已生成**：[固定 artifact](https://github.com/yuxino/mimi/actions/runs/36734613924/artifacts/11107067102)，归档 SHA256 `dd7781f7a27fe0a7c0242546d070a11e57b7cb1296f4af8411ac45c902181173`（不是内部 AppImage 哈希）。deb/AppImage 各三次自动安装/启动回归通过；远程 Mac Rust 520 passed/1 ignored，两条新增 fixture 回归通过。发布步骤跳过。云QA已经用该包和隔离临时XDG/上述两个开关补验取消、保存与路径重试；后端磁盘写入失败仍未测，旧 `197f165` 包不能验证新入口。
- **云 QA 的 `197f165` Linux 原生 UI-only 结果**：DeepLX 上述高级配置通过；合成字幕显示/暂停保留/恢复、两个动效开关、panel 诚实无音频、toast 出现/消失通过，测试应用已正常停止。导出受停止清空 archive 的测试 fixture 阻塞，两次显示 Nothing saved；这是验收入口缺口，不是生产丢数据证据。旧组合迁移、全七态时序、系统 reduce、流式多句/长句未覆盖；单张静图不能证明这些路径。

- **此前 head `197f165446e802b1fa4d3717142c4cf626142b3e` 十项适用 CI 全绿**：[桌面/跨平台](https://github.com/yuxino/mimi/actions/runs/36730109016)、[Android](https://github.com/yuxino/mimi/actions/runs/36730108479)。包括前端、三平台 Rust、MSRV、三平台 bundle、Windows ARM64 和 Android；发布相关步骤全部跳过。
- **同一 head 的 Linux 自动包检查通过**：Ubuntu 22.04 CI 中实际安装 deb，并分别启动 deb/AppImage 各三次。隔离 XDG/DBus/Xvfb、UI-only 合成会话下，设置与字幕渲染、窗口可见、精确几何/移动跟随、停止/重启及关闭退出全部通过。[实际日志](https://github.com/yuxino/mimi/actions/runs/36730109016/job/109937146007)。这不替代新动效七状态的 Linux 原生视觉验收，也未验证真实服务或物理音源。
- 此前已验包为[197f165 的 Linux CI 包](https://github.com/yuxino/mimi/actions/runs/36730109016/artifacts/11104407599)，artifact archive SHA256 `4bbb1f4350b4ca907dfe9543a71358b8ce3acaafabd0fb4c89270ebdeb79b551`。这里是下载归档的哈希，不是其中单个 AppImage 哈希；整合者本轮未下载/安装/启动；后续导出补验须用 `38bc4df` 新 CI 包并重新记录归档/可执行文件哈希。此为此前暂停阶段的记录；用户随后已明确授权独立 dev 候选安装，真实凭据与权限交互仍不代批准。
- **此前 DeepLX head `fcdc1a985d245e4380522615af32bdbe1f5b9dc3` 十项适用 CI 全绿**：[桌面/跨平台](https://github.com/yuxino/mimi/actions/runs/36725662886)、[Android](https://github.com/yuxino/mimi/actions/runs/36725662119)。前端 224 tests/31 files、typecheck/build通过，lint 0 error/1 既有 warning；三平台 Rust 完整检查、MSRV、三平台 bundle 与 Windows ARM64 均通过。ARM64 实际日志 522 passed/2 ignored，release 编译完成；发布相关步骤跳过。高级布局同条件图片与固定包原生验收仍待完成。
- **此前 head `42f758378e0d2b47bc65922ac105f0907f5a62f1` 十项适用 CI 全绿**：[桌面/跨平台 CI](https://github.com/yuxino/mimi/actions/runs/36715331676)、[Android CI](https://github.com/yuxino/mimi/actions/runs/36715331394)。前端 214 项测试/30 文件通过，lint 0 error/1 既有 Fast Refresh warning，typecheck/production build 通过；Rust 三平台、MSRV、Windows ARM64 编译及 UI-test 启动、三平台 bundle 均通过。发布相关步骤跳过。原生同包与用户视觉验收仍待完成。
- **旧范围、未含 #67** 的 `0455e13`：十项 CI 全绿，见 [桌面/跨平台](https://github.com/yuxino/mimi/actions/runs/36711466913)、[Android](https://github.com/yuxino/mimi/actions/runs/36711466043)。此前 aggregate `scripts/check.sh` 509 Rust（1 ignored）、197 前端及格式/lint/typecheck/build通过。
- 旧 local Android 离线重跑因缺少 Gradle plugin 8.7.3 缓存失败，未记为通过。用户要求减负后没有补跑本机编译。
- **统一包完整原生验收尚未完成，暂不合并。** 正式候选已按用户授权短测并回滚，首次凭据读取请求认证；正常授权/两次冷启动待验。dev 暂停。顶部记录精确版本与后续路径，不用 UI-only、CI 或安装事实替代。
- 最新 Mac/Linux 的合成暂停恢复、重启偏好及原生导出保存/取消已通过（准确版本见正文顶部）。字幕收起/展开/错误/流式长句、系统 reduce/七态、真实服务首字幕及物理硬件仍待验；未发版、打 tag 或部署。

### 下一步验收

1. 明早通过隔离本机候选看桌面三屏图示，用户最终视觉确认后才考虑将其纳入生产；正式凭据与权限认证等用户在场。
2. Android 新文案专项 9 张三语 API35 原图已完成；真实采音与首字幕仍待验。
3. 系统 reduce/七态、流式多句/长句、全局快捷键焦点、持久历史删除恢复与平台物理路线按未验边界继续跟踪；不把安装和 CI 当整包全通过。

本轮不合并、发版或关闭 issue。

## issue 关闭规则

只在集成合并且对应验收完成后关闭用户自己开的 issue，并贴 PR/证据；不用宽泛自动关闭关键词。

| issue | 作者 | 当前处理 |
| --- | --- | --- |
| #77 历史导出/删除 | yuxino | 新 Mac/Linux 合成原生导出取消/保存已验，Mac 当前记录清空已验；持久历史删除恢复未验、集成未合并，保持开放 |
| #75 安全诊断 | yuxino | 统一包复制/隐私与凭据状态证据未齐，保持开放 |
| #74 Windows 音源、#78 跨平台采音 | yuxino | 物理路线未验，保持开放 |
| #82 首次使用 | yuxino | PC #85 HOLD、真实首字幕未验，保持开放 |
| #83 钥匙串弹窗 | yuxino | 根因未解决，保持开放；未将安全分类补丁当作认证修复 |
| #87 字幕细节 | yuxino | 用户已选择保留A/B；Linux原生三样式/预览浮层同步/暂停恢复/静态/切页已验并补图，Linux 新包原/A/B 重启持久化已验；系统reduce、七态仍待验，保持开放 |
| #70 自定义 API | 294910541 | 外部作者、DeepLX 只是部分覆盖，不自动关闭 |

#67 关联 #48（yeshuoer）、#59/#61（LLLin000）已关闭，本次不改变状态。最小字体及更大布局预览不在本 PR 中。 [完整来源与验收记录](https://github.com/yuxino/mimi/blob/26894aebc4264ee563fd679a001f048e118587c0/docs/qa/2026-09-30-integration.md)。


