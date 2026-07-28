# mimi 本机稳定签名设计

## 目标与方案

mimi 需要 ScreenCaptureKit 的屏幕与系统音频录制权限。临时签名会随二进制内容变化而改变代码身份，导致 macOS TCC 无法把新的开发构建与已授权版本稳定关联。解决方案是在用户登录钥匙串中保存一个仅用于本机开发的自签名代码签名身份 `mimi Local Development`，私钥不导出、不提交，也不上传到任何服务。

打包脚本按以下顺序选择身份：显式提供的 `MIMI_CODESIGN_IDENTITY`、登录钥匙串中的 `mimi Local Development`、最后才是临时签名 `-`。这保留了其他开发机器的可构建性，同时让当前 Mac 的连续构建拥有相同的指定要求。证书只被用户信任用于代码签名，应用仍使用原有 bundle identifier `app.yuxino.mimi`。

验证分为三层：钥匙串必须把本机证书列为有效代码签名身份；连续两次打包得到的 `codesign -d -r-` 指定要求必须一致；最终应用通过 `codesign --verify --deep --strict`。重新签名后的版本需要用户最后授权一次 Screen & System Audio Recording，之后只要继续使用相同证书、bundle identifier 和路径，后续开发构建就能沿用该权限。
