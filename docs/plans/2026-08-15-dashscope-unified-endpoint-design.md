# 迁移到 DashScope 统一端点（去掉 Workspace ID）

> 2026-08-15。发现 koe（浏览器同传扩展）用 `dashscope.aliyuncs.com` 统一端点、只需 API Key 即可翻译，而 mimi 一直用老版 MaaS 端点（`{workspace}.cn-beijing.maas.aliyuncs.com`），必须填 Workspace ID。本记录说明迁移原因与改动。

## 结论（一句话）

**DashScope 提供统一域名端点，认证只靠 `Authorization: Bearer <api-key>`，不再需要 Workspace ID。** mimi 的三个客户端从 MaaS 端点迁到统一端点后，设置里可以去掉 Workspace ID 输入，用户只需填 API Key。

## 新旧端点对照

| 客户端 | 旧端点（MaaS，需 workspace id） | 新端点（DashScope 统一，仅 API Key） |
|---|---|---|
| Audio3 ASR | `wss://{ws}.cn-beijing.maas.aliyuncs.com/api-ws/v1/inference` | `wss://dashscope.aliyuncs.com/api-ws/v1/inference` |
| LiveTranslate 实时翻译 | `wss://{ws}.../api-ws/v1/realtime?model=...` | `wss://dashscope.aliyuncs.com/api-ws/v1/realtime?model=...` |
| Qwen-MT HTTP 翻译 | `https://{ws}.../compatible-mode/v1/chat/completions` | `https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions` |

## 依据

- koe 源码（`offscreen.js`）：ASR 连 `wss://dashscope.aliyuncs.com/api-ws/v1/inference/`，翻译 POST `https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation`，都只带 `Authorization: Bearer <key>`。
- 官方 LiveTranslate 平台文档：`wss://dashscope.aliyuncs.com/api-ws/v1/realtime`（国内站）。
- koe 的 ASR 请求体（run-task / task_group audio / function recognition / qwen-audio-3.0-asr-flash-streaming / pcm 16000 / semantic_punctuation / heartbeat）与 mimi 的 audio3 协议**逐字段一致**，只有 URL 不同——迁移零协议风险。

## 改动

1. **协议层**（`core/protocols/`）：三个 endpoint 的 `new()` 去掉 workspace_id 参数，URL 改为统一域名常量（`DASHSCOPE_INFERENCE_WS` / `DASHSCOPE_REALTIME_WS` / `QwenMTEndpoint::URL`）。
2. **客户端层**（`clients/`）：`Audio3ASRClient::new`、`LiveTranslateClient::new`、`QwenMTClient::new`、`HighQualityTranslationClient::new` 去掉 workspace_id 参数。
3. **配置**（`core/configuration.rs`）：`validated()` 不再要求 workspace id；删除 `WORKSPACE_ID_PATTERN`、`is_valid_workspace_id`、`InvalidWorkspaceID`/`MissingWorkspaceID` 错误。`workspace_id` 字段保留（可选，兼容旧设置迁移）。
4. **设置 UI**（`SettingsView.tsx`）：移除 Workspace ID 输入框；`credentialsAreConfigured` / `showsServiceSettings` 只依据 `hasAPIKey`；保存只传 API Key。
5. **测试**：更新 endpoint URL 断言；删除 workspace 校验测试；新增「无 workspace id 也能 validated」测试。

## 未改动

- 请求/响应**协议体**（run-task 参数、translation_options、session.update 等）完全不变——只换 URL 域名。
- Qwen-MT 继续用 `compatible-mode/v1/chat/completions`（chat 格式 + translation_options），未切到 koe 的 generation 端点（请求体结构不同，chat 格式已验证可用）。
- 前端 `SettingsSnapshot.workspaceID` 字段保留（后端 payload 仍含，兼容旧版本保存的数据）。

## 验证

- `cargo test`：147 个 Rust 测试通过（含新增的 workspace-less 配置测试）。
- `./scripts/check.sh` 全绿。
- 真实会话需用用户 API Key 实测（自动识别 + 实时翻译 + 高质量翻译三个模式）。
