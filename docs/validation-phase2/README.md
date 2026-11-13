# 阶段 2 当前实例证据

基线见 BASELINE.txt，原始资料均为合成测试，不包含真实用户数据库、Key、私钥或配对 token。

- rust-tests.txt：72 项 Rust 通过，含真实 loopback TLS、两种私钥证明、配对、两个 SQLite 副本、迁移/回滚和旧任务子记录；不是实机或家庭网络结果。
- python-tests.txt：10 项；ui-results.json：18 项显式 UI Mock；二维码/状态 TypeScript 单元测试为 3 项。
- clippy.txt / linux-tauri.txt / android-plugin-rust.txt：当前实例编译检查。
- local-unsigned-android.json / android-build-summary.txt：未提交工作区的本地未签名完整 Android 构建及结构验证。base_commit 仅标记基线，**不是最终已提交包源码承诺**；签名未验证、不可安装、不可交付。
- dependency-preservation.json：现存外部依赖版本没有被替换；schema.json / branding.json：数据库与品牌资源结果。
- sync-*-mock.png：明确的 UI 夹具截图，不能冒充原生同步或用户设备验证。

Windows Hosted 构建、签名 APK、Release 和公开下载验签仍需实际 Actions/签名条件完成后补充，不能据这些本地文件计为通过。业务记录、签名 keystore 和安装包不提交到此目录。
