# 版本与阶段发布规则

`VERSION.json` 是统一版本源，`VERSION_HISTORY.json` 仅记录实际已发布的版本与源 SHA；失败构建不加入历史。新版本必须是最后已发布版本的下一值，versionCode 必须大于历史最大值；维护当前已发布版本的文档时不能改变该版本的 code。当前开发目标是 **0.0.2 / Android versionCode 1002**；此文件不表示已经发布。
已发布历史：0.0.1 / 1001；不得覆盖 `v0.0.1`。

顺序为 `0.0.1 … 0.0.9 → 0.0.10 → 0.1.0 … 0.1.10 → 0.2.0`；第二位从 9 进位时第一位加一：`0.9.10 → 1.0.0`。末位允许 0～10、第二位允许 0～9，不使用预发布后缀替代阶段编号。
`python3 scripts/version.py --next 0.0.10` 只计算下一值，不分配版本、创建 Tag 或发布。
`python3 scripts/version.py` 检查 npm 工作区、锁文件、Rust 包、Tauri 与 Android 版本和应用身份一致；测试覆盖进位与非法版本。

每阶段的 Windows 与 Android 构建必须检出同一完整 commit SHA，使用同一 versionName。Android versionCode 单独严格递增；发布前须与最近已发布 APK 对照，不可只根据版本字符串推算或回退。
构建失败、缺少签名或未能验证安装包时，不创建正式 Tag、不公开 Release、不占用下一版本号；修复后重试同一未发布目标。

CI 仅测试、构建和保存 Artifacts。`release.yml` 只接受手动阶段发布授权及完整 main commit SHA，等待测试和双端构建全部成功，校验包与 SHA 后上传草稿，最后公开 Pre-release 并创建 Tag。历史 Tag/Release 存在则拒绝覆盖。构建任务仅有 contents:read；公开阶段仅有 contents:write。

用户明确说“开始阶段 N”时，授权该阶段开发、测试、提交、推送、Actions 双端构建及对应测试版发布，不授权下一阶段。每次发布后更新开发计划、变更记录、版本历史、阶段验收与构建报告。普通讨论和局部咨询不构成发布授权。

2026-11-14 CI 修复仍使用未发布目标 0.0.2 / 1002。本地检查通过不分配新版本；须验证同 SHA 的 Windows EXE 与原签名 Android APK 后才创建 v0.0.2 Pre-release。

本次接管核查和文档校正不启动新版本：开发目标继续为未发布的 0.0.2/1002，已发布历史仍只有 0.0.1/1001。VERSION_HISTORY.json 只在真实发布完成后更新，不因 CI 构建成功或文件缺失预先占用版本。
