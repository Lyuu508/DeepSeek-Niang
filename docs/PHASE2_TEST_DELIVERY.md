# v0.0.2 阶段 2 独立测试交付

日期：2026-11-14（Asia/Shanghai）。业务代码基于 GitHub main 9cc9a16；保留随后只更新状态文档的 7acd099 历史，未 reset/覆盖 main。目标继续 0.0.2 / Android versionCode 1002，不创建正式 v0.0.2 Release 或新版本。本报告记录提交前的实际本地结果；最终同源码 CI Run 与 Artifact 下载链接以本次交付答复为准，远程包内各自报告记录完整 source_commit。

## 本次实现

- Android debug 安装身份固定为 io.local.deepseekcompanion.test，启动名称 DeepSeek娘 测试版，原正式 release 身份 io.local.deepseekcompanion 不变。原图及69项品牌资源不变。
- 新增 sh --test-package / PowerShell -TestPackage。应用内显示独立测试提示；SQLite 与 Android Keystore 按不同应用沙箱/实际 packageName 隔离，不自动读取、导入或清理旧正式版数据。手动跨设备配对仍会经双方确认共享资料，应使用合成测试样本。
- 仅在显式构建测试包时准备开发测试签名，Keystore 留在被忽略的 gen/android/.test-signing，目录0700/文件0600；私钥通过证书请求操作在内存中验证可用，不导出。证书不得与旧升级证书相同，不将 Keystore、密码或私钥放入 Git/Artifacts。不同 Runner 的开发签名可能变化，不保证旧测试包可直接覆盖，不要求卸载正式版。
- 去除测试 Rust 原生调试符号，保留开发测试变体，不改变同步、TLS或数据库规则。清理的是重新打包过程的生成 APK：旧生成 APK 完整保存在 checkout 外，没有删除用户文件/应用数据；本地新包约54 MiB。
- 实际 APK 校验包含 appId/启动名、0.0.2/1002、仅ARM64、ZIP、真实 apksigner、无私钥文件、禁自动备份/明文流量、无共享 UID，以及 debug/upgrade 区分。生产升级校验器会拒绝 .test 包；正式收集器也要求原 ID、upgrade 类型、原证书，不能因测试签名有效就公开升级包。
- 无参数 Android 构建仍只接受原签名 release；保留显式 unsigned-ci 检查模式。PowerShell 原先直接 debug 构建正式 ID 的路径已改为显式测试或原签名模式。按 debug/release 目录选择当前变体，避免本地旧 APK 混入新结果，不删除另一变体输出。
- Windows 和 Android CI 各自独立上传包、SHA256SUMS、报告和构建日志；核对 HEAD 等于同一 EXPECTED_SOURCE_SHA 且工作树干净。Android 问题不取消 Windows。
- Linux CI 保留 Rust、Clippy、Python、数据库、版本、品牌、前端以及两种 UI 标记模式的测试日志/报告。没有减少、忽略或删除原测试。

未修改同步协议、发现实现、TLS要求、生产 applicationId、数据库迁移或业务数据。阶段 2 的生产同步已在既有提交实现，本次补上可安装的安全测试交付通道，而非重新开发原功能。

## 已实现与验证层次

| 功能 | 已实现 | 自动化/构建 | 真实设备 |
|---|---|---|---|
| Windows主机/Android手机 LAN 与网线PC+Wi-Fi拓扑 | 监听私有IPv4、mDNS/NSD、手动地址回退已实现；无云数据库 | 真实TCP/TLS自动化及既有原生编译；发现/系统凭据夹具不当作原生发现 | 未执行，待用户家庭网络 |
| 安全配对、双方确认、TLS1.3、身份固定与撤销 | 已实现 | 原有真实loopback TLS、客户端证书、两种私钥证明、坏证书/撤销拒绝测试保留通过 | 未执行 |
| 聊天/人格/记忆、SQLite增量、断线恢复、冲突/删除 | 已实现 | 原73项Rust含迁移/回滚、真实SQLite、分页/去重/离线补传；部分服务为同进程+测试Vault/NoDiscovery | 未执行 |
| Windows安装程序 | 既有构建与安装smoke路径保持 | 9cc9a16 与7acd099既有CI已成功；本次提交后重新跑Windows原生构建 | 用户Windows未验收 |
| Android独立测试包 | 本次补齐 | 本地实际APK生成、验签/身份/ABI/Manifest通过；本次提交后重新跑Hosted Android | 未安装到物理手机 |
| 原签名升级与正式发布 | 独立待办，本次不恢复/迁移 | 不计通过，不运行release.yml | 待后续 |

用户已明确没有可供代理连接的 Windows/Android 设备，交付后由用户验收。本机是 Linux，ADB 默认用户目录只读，且没有提供家庭双端通道；没有把 Mock、APK编译或Windows安装窗口当成真实双端LAN验收。

## 实际本地检查

详见 [完整证据](validation-phase2-isolated-test/)。

- Rust：73项通过、0失败、0忽略；零测试目标不计数，原有失败场景与回归均保留。
- Python：11项通过，新增测试验证 debug 后缀只作用于测试身份、正式ID/名称不变且准备脚本幂等。
- 同步模型/二维码：3项通过。
- 浏览器：常规与测试标记模式各18项通过（共36次执行、18个原有用例），明确是UI夹具，不是真实DeepSeek或系统凭据验收。
- Clippy、Rust格式、TypeScript/Vite、SQLite完整性/外键/FTS/人格归属、69项品牌、版本、sh语法、actionlint和diff空白检查通过。
- 本地真实 .test APK：56,616,736字节，SHA-256 b6b897798cc65c3d087671555b96bac1ccf2f17dfb1c1e368091a5fa92d35a2e，真实开发签名验证通过、原签名不兼容。该数值仅标记提交前诊断包，不冒充最终CI包。
- release回归：实际构建原ID的未签名release APK并检查通过，仍是0.0.2/1002、正式启动名、无debug标记、禁备份/明文；不能安装交付，也不是正式升级包。

首次测试构建因尚未创建显式开发签名文件失败；添加隔离测试签名准备后完整重跑通过。首次debug包约220MiB；去掉原生调试信息并重新生成APK归档后约54MiB（旧增量APK保存在缓存）。失败/优化/回归日志均保留，不能把首次失败算通过。本地报告 working_tree_clean=false 是尚未提交的验证工作区，不能作为最终同SHA交付证明；CI从提交检出干净源并强制核验。

PowerShell Android入口已对齐，但本机没有PowerShell/Windows Android工具链，本地Android实际执行的是Linux脚本；Windows EXE应以提交后的Hosted Windows实际结果为准。

## 安装包交付与验收

不创建任何正式v0.0.2 Tag/Release。提交后既有CI分别构建Windows EXE和独立Android测试APK，Artifact名称 windows-package-1、android-package-1、automated-test-reports-1（重试时后缀变化），每份保留30天。Windows包含安装器与应用EXE、SHA256SUMS、windows-build.json、安装启动报告/截图及构建日志；Android包含明确命名的 isolated-test.apk、SHA256SUMS、android-build.json和完整构建/工具链日志。

这是GitHub Actions ZIP下载，需要登录有权限的GitHub账号，不是正式Release公共链接。之前实际下载被productionresultssa30.blob.core.windows.net与productionresultssa8.blob.core.windows.net限制；本次不改白名单、不绕过限制。远程附件不能下载时，报告CI作业实际状态及限制，不虚称已独立复验远程二进制内容。

收到包后按 [PHASE2_ACCEPTANCE](PHASE2_ACCEPTANCE.md) 在Windows独立测试账户与Android测试应用执行同路由器网线+Wi-Fi发现/双方确认、双向聊天/人格/记忆、断网重启补传、增量/重复、字段冲突、删除/撤销、凭据与生命周期。确认原v0.0.1仍可打开且旧数据保留。不要上传真实数据、Key、私钥或配对码。

原签名恢复、RSA文件传输和Secrets迁移本次不执行，只保留独立待办。实机项目未验收前不勾选完成，等待用户验收后再决定阶段3。
