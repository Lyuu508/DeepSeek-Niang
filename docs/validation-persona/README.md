# dev.2 本次验证证据

2026-11-12，Linux 环境。原始阶段0/1与安装包记录不覆盖，单独保存本次结果。用户真实数据库和原始角色PNG未进入环境。

| 文件 | 实际检查 |
|---|---|
| rust-tests.txt | `cargo test --workspace --offline --locked`，57通过，包含原21项 |
| clippy.txt | `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` |
| frontend-build.txt、typecheck.txt | `npm run build` / `npm run typecheck`，共享前端 |
| browser.txt、results.json、persona-*-mock.png | `npm run test:ui`，16项明确浏览器Mock，Chromium156 |
| policy-tests.txt | 6项Python策略/资源夹具，未使用正式原图 |
| schema.json | 全部迁移DDL、人格归属/外键/范围/中文FTS/完整性 |
| android-rust-check.txt | `cargo check --manifest-path apps/companion/src-tauri/Cargo.toml --target aarch64-linux-android --offline --locked` |
| windows-vault-check.txt | `cargo check -p companion-secrets --target x86_64-pc-windows-msvc --offline --locked` |
| packaging-gate.txt | 缺少原始PNG时品牌校验以非零退出，预期阻止打包 |
| windows-packaging-attempt.txt、android-packaging-attempt.txt | 实际执行两端Tauri打包入口，均在缺原始PNG的beforeBuildCommand退出1，无新版产物 |
| baseline-preservation.json | 旧DDL、原21项测试及关系/任务/视频源文件按字节保留 |
| dependency-preservation.json | npm/Cargo外部依赖锁条目未变化，无新增程序/模型/付费服务 |
| summary.json | 当前版本、结果计数与未完成范围 |

fmt（workspace及native manifest）、sh -n、Python编译、Node语法检查通过。生产JS经搜索不含fixture回复/测试密钥/延迟读取夹具代码。

Rust命令复用已有工具链/缓存，CARGO_TARGET_DIR仅指向本轮工作缓存。Android检查设置已有NDKr28c的aarch64-linux-android29-clang与llvm-ar；没有安装新依赖。跨检查不是Windows或Android运行。APK的Kotlin/AAPT/Gradle、覆盖升级、Keystore/Credential Manager、真实DeepSeek、全时间线视频、桌宠/语音均不能由本目录证明。

完整范围和缺项见 [本次报告](../PERSONA_BRANDING_REPORT.md)。
