---
name: mimi-dev-launch
description: 启动 mimi Tauri 应用进行开发/查看时使用。当用户说"启动看看"、"跑一下"、"打开应用"、验证 UI 改动等场景时使用。
---

# mimi 开发启动

## 核心规则

启动 mimi 应用用于开发或查看时,**必须**用 `./scripts/dev-app.sh`,**不要**用 `npm run tauri dev`。

## 为什么

- `npm run tauri dev`(debug 构建,`!custom-protocol`)会触发 Tauri 运行时 Dock 图标覆盖,把 Dock 图标替换成**未蒙版的方形图标**(macOS 不套圆角蒙版)。这是已知坑,详见 `docs/plans/2026-08-14-tauri-dev-dock-icon-design.md`。
- `./scripts/dev-app.sh` 用与 `tauri build` 完全相同的构建路径(`cargo build --release --features tauri/custom-protocol`),Dock 图标走 bundle icns + 系统蒙版,圆角正确;并打包成 `mimi-dev.app` 用稳定本地签名重签,保留屏幕录制/keychain 授权。

## 使用步骤

```bash
./scripts/dev-app.sh
```

- 会自动 pkill 旧 dev 实例、`npm run build` 前端、release 构建、组装并启动 `src-tauri/target/release/mimi-dev.app`。
- 窗口标题/托盘提示带 "(dev)" 标记,与 release 应用区分。
- 首次或改动 Rust 时构建较慢,建议后台运行并等待输出。

## 打包 release(仅当用户明确要求)

默认**不要**打包 release。只有当用户明确说"打包"、"做个安装包"、"发布"之类才运行:

```bash
./scripts/package-app.sh
```

- 产物在 `src-tauri/target/release/bundle/`,macOS 下是 `macos/mimi.app` + `dmg/*.dmg`。
- 会重新签名,并可手动 `cp -R` 到 `/Applications` 替换安装。

## 检查环境变量(可选)

- `MIMI_UI_TEST=1`:注入演示凭据,方便直接看界面。
- `MIMI_AUTO_START=1`:启动时自动开始会话。
