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
| Windows安装程序 | 既有构建与安装smoke路径保持 | 1c86678 的Hosted Windows原生测试、EXE和安装/窗口smoke全部通过 | 用户Windows未验收 |
| Android独立测试包 | 本次补齐 | 本地实际APK检查通过；1c86678 的Hosted Android构建、验签/身份/ABI/Manifest与源SHA检查全部通过 | 未安装到物理手机 |
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

## 最终同源 CI 交付结果

2026-11-14 14:54（Asia/Shanghai），[Run 43567948075](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43567948075) 的三个作业全部成功，源码均为 **1c86678963cb83819e7a72cada6d279fcac9736c**。本节随后的文档提交只记录结果，不能冒充该源码的安装包。源SHA、作业步骤和Artifact元数据均已由GitHub API核实，快照见 [ci-delivery.json](validation-phase2-isolated-test/ci-delivery.json)。

- Windows：原生cargo test、EXE构建、NSIS安装、窗口启动、源码一致/干净检查及SHA256SUMS生成/上传均通过。[Windows安装包及校验/报告/日志](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43567948075/artifacts/14102788404)。Windows安装器未做Authenticode签名。
- Android：实际生成 DeepSeek-Niang_0.0.2_android-arm64-isolated-test.apk，约54MiB，.test身份、开发签名、正式包不兼容和安全Manifest校验均通过，sourceSHA/干净源码检查通过；原签名恢复步骤按测试模式跳过。[APK及SHA256SUMS/报告/日志](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43567948075/artifacts/14102914238)。CI开发证书由该Runner生成，不能用本地诊断证书或哈希替代其包内报告。
- Linux：Rust/Clippy/版本/DDL/Python/同步模型/前端/品牌以及常规与测试标记两个UI模式全部成功。[自动化测试报告和截图](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43567948075/artifacts/14102854124)。测试范围仍按上文区分，不计为物理双端。

以上是需登录GitHub下载的真实Actions ZIP，各安装包ZIP分别含SHA256SUMS.txt和本平台构建报告/日志，不是虚构的Release文件链接。当前会话实际尝试下载三个远程附件时，均被 productionresultssa8.blob.core.windows.net 拒绝；没有修改白名单、绕过限制或输出带临时签名的URL。故确认CI受检步骤/上传成功与包来源元数据，不声称本会话已取回并独立复验远程二进制内容。

GitHub仍仅有历史v0.0.1 Release，未创建v0.0.2正式Tag/Release。Windows/Android物理设备与家庭网线+Wi-Fi实测仍未执行，用户已确认收到包后自行验收。原签名恢复/Secrets保持独立待办，阶段3未开始。

## 最新授权：独立文件公开到 Releases

用户随后明确要求以与0.0.1相同方式公开测试版，故 v0.0.2 Pre-release 现已发布，而非继续等待或仅提供Actions ZIP。下载入口见 README 和 docs/RELEASE_0.0.2.md。两端仍为同一源码1c86678的原CI产物，没有重新签名/编译或冒充原身份升级。发布运行43571208371成功，Release ID445111880，Tag指向1c86678963cb83819e7a72cada6d279fcac9736c；安装包、校验/报告及日志13项公开附件均已实际下载HTTP200并比对SHA-256，APK用官方apksigner独立复验通过。

本文件之前“不创建Release”的描述为上一次授权阶段的历史结果，已由本次用户明确公开授权替代。物理双端仍未验收，阶段3未开始，原签名恢复/Secrets仍独立待办。完整CI编译输出保存在原始CI 43567948075，附件windows-build.log为安装构建流程的PowerShell transcript，不将其短文本等同于全部cargo编译输出。
