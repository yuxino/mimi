# mimi 本机稳定签名设计

## 目标与方案

mimi 需要 ScreenCaptureKit 的屏幕与系统音频录制权限。临时签名会随二进制内容变化而改变代码身份，导致 macOS TCC 无法把新的开发构建与已授权版本稳定关联。解决方案是在用户登录钥匙串中保存一个仅用于本机开发的自签名代码签名身份 `mimi Local Development`，私钥不导出、不提交，也不上传到任何服务。

本地脚本按以下顺序选择身份：显式提供的 `MIMI_CODESIGN_IDENTITY`、登录钥匙串中唯一的 `mimi Local Development` 指纹。显式值可以是系统能解析的身份名或指纹，建议使用精确指纹；自动选择遇到重名身份时则会失败。缺少稳定身份时开发启动与 macOS 打包都 fail closed，不再生成会重置权限的临时签名包。证书只被用户信任用于代码签名。

开发版在编译配置与 bundle 中都使用独立 identifier `app.yuxino.mimi.dev`，固定安装到 `/Applications/mimi-dev.app`（可显式覆盖为另一个长期不变的绝对路径），不会覆盖正式版。Tauri 配置目录和系统钥匙串 service 也按该 identifier 隔离，开发调试不会读取或修改正式版档案与凭证。验证分为三层：钥匙串必须把本机证书列为有效代码签名身份；连续两次构建得到的 `codesign -d -r-` 指定要求必须一致；最终应用与实际启动进程都必须通过路径、bundle identifier 和 `codesign --verify --deep --strict` 校验。首次切换到该稳定开发身份仍需授权一次，之后连续构建沿用同一 TCC 身份。该自签证书没有 Apple Team ID，因此钥匙串的 partition 检查仍可能在二进制 CDHash 改变后要求一次授权；这不是删除重建凭证或放宽 ACL 的理由。跨构建完全无提示需要迁移到 Apple 签发且 Team ID 稳定的身份。

## 实现状态（2026-08-14）

方案已落地：

- `scripts/codesign-identity.sh`：优先接受显式 `MIMI_CODESIGN_IDENTITY`（建议使用指纹），否则选择唯一 `mimi Local Development` 证书的精确指纹；没有稳定身份时返回不可用，调用方必须 fail closed。
- `scripts/package-app.sh`：在 `tauri build` 前把所选身份交给 Tauri，使 `.app` 与随后创建的 `.dmg` 使用同一个本机身份；构建后通过 `codesign --verify --deep --strict`。GitHub 正式包另用独立且长期固定的自签发布身份。
- `scripts/verify-macos-install-identity.sh`：替换正式应用前比较完整 designated requirement；本地开发身份与 GitHub 发布身份不同时默认拒绝，证书迁移只能显式放行并接受一次新的 TCC/Keychain 授权。
- `src-tauri/tauri.dev.conf.json`：为开发构建提供独立 product name 与 identifier；设置目录和开发钥匙串 service 不与正式版共享。
- `scripts/dev-app.sh`：只接受稳定身份；把 dev 配置注入带 `tauri/custom-protocol` 的真实 `.app`，严格验签后通过同卷 staging + 回滚安装到固定路径，并用实际 executable path 验证唯一启动进程。安装期间用 `lockf` 阻止并发替换，恢复失败时保留备份。
- 开发启动前会拒绝仍在运行的其他 mimi 正式版或开发版，避免旧副本继续占用全局快捷键并以另一身份触发权限请求。
- `--ui-only` 仍使用同一个稳定 bundle，但应用层保证不访问凭证、网络或系统音频，因此可用于无授权 UI 检查。
- 不再建议在 macOS 上运行任何 `tauri dev` 命令或裸 `target/*/mimi`；这些 ad-hoc 二进制的指定要求随构建变化。Windows 使用 `npm run tauri:dev` 取得同样的数据隔离。
- 本地 `app.yuxino.mimi` 包仍由 `mimi Local Development` 签名，不是 GitHub 正式版的兼容升级包。日常预发布验证固定使用 `/Applications/mimi-dev.app`，避免在两个正式身份之间来回覆盖并反复触发系统授权。
