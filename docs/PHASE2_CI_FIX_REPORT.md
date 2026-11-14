# 阶段 2 双端 CI 修复报告

日期：2026-11-14（Asia/Shanghai）。修复基线：`b7330d99b587810ca394ae231b0e184a7131acde`。
用户提供失败 Run `43484379215`，Windows Job `132088816867`、Android Job `132088816687`、Linux Job `132088816383`。当前代理拒绝 api.github.com，未能直接读取该 Run 日志；原始失败现象来自用户报告，下列诊断另由本地真实通信测试复现。

## 根因与修改

Windows：SyncService 将监听器设为非阻塞，但 incoming 使用阻塞式 rustls::StreamOwned。Windows 的 accept socket 继承非阻塞模式；在 ClientHello/后续数据未就绪时，read 返回 WouldBlock，被 transport 映射为 Offline。Linux accept 通常不继承该模式，因此原测试在 Linux 通过而不能覆盖 Windows 情形。

修复只在连接 worker 开始时显式 set_nonblocking(false)，仍保留 8 秒读写超时、私有地址限制、TLS 1.3、证书验证/固定、客户端证书认证和双方确认。没有删除、忽略或替换原有测试。
新增 accepted_nonblocking_socket_waits_for_real_tls_client 在任意平台强制 accepted socket 非阻塞，延迟真实 ClientHello，并用真实 TCP、服务端证书固定、客户端证书和 TLS 1.3 完成同步请求。未改服务代码前确定失败为 Offline；修复后通过，前后原始日志均已保存。这不替代 Windows 原生执行结果。

Android：原工作流直接调用不保证在 PATH 中的 sdkmanager。GitHub runner-images Ubuntu 24.04 文档显示预装 SDK/Command-line Tools，但不能据此假设命令已暴露或所需 NDK 已装。
新增固定 SHA 的 android-actions/setup-android v3，选择 Command-line Tools 16.0（12266719），由 action 配置工具路径；随后仍真正执行 sdkmanager 安装 Platform 36、Build Tools 36.0.0、platform-tools、NDK 28.2.13676358。检查 JDK 21、sdkmanager --version、SDK/NDK 实际路径、android.jar、apksigner、aapt2、ARM64 编译器、NDK source.properties 与 installed package 清单，保存安装日志。ANDROID_HOME / ANDROID_SDK_ROOT / NDK_HOME 与工具 PATH 写入后续步骤环境。未通过跳过 SDK 安装消除错误。

签名：重新下载公开 v0.0.1 APK，用官方 Build Tools 36.0.0 的真实 apksigner verify --verbose --print-certs 及 aapt2 验证。Build Tools 下载包校验了 Google HTTPS repository2-1.xml 中的官方归档校验和。

- APK SHA-256：5100cdd29b6751e3ffd1d6536f5c125af6888f679dc524cdfbb6048afe3195d8。
- v2 签名通过；唯一签名证书 SHA-256：9de96ea5fb1d0b3f91ea73d9a16c030428453b15838927a85f86773eaef208a1。
- applicationId：io.local.deepseekcompanion；versionName 0.0.1；versionCode 1001。
- scripts/android_signing.py 的预期证书与实际输出相符，保留原值和现有 keystore 校验。签名发布 job 新增历史 APK 实时验签，随后才恢复 Secrets。产物验证要求恰好一个匹配签名者。
- 尚未确认四项 Actions Secrets 是否存在：API 请求被代理拒绝，不据此判断它们不存在。没有读取 Secret 值、生成替代密钥或更改应用身份。

## 自动化检查与证据

[原始日志目录](validation-phase2-ci-fix/)。本次修复工作区实际执行：

| 检查 | 结果 |
|---|---|
| cargo test --workspace --locked | 73 项通过、0 失败、0 忽略；含原三个失败场景和新增真实 TLS 回归；零测试目标不计数 |
| cargo clippy --workspace --all-targets --locked -- -D warnings | 通过 |
| cargo fmt --all -- --check | 通过 |
| npm run build | 类型检查和生产构建通过 |
| Python unittest | 10 项通过 |
| node --test tests/sync-model.test.mjs | 3 项通过；在 Rust 生成二维码后执行 |
| npm run test:ui | 18 项通过，明确 UI Mock，不代表原生/API/实机 |
| check_schema / check-branding / version | 完整性与约束、69 项品牌哈希、0.0.2/1002 版本一致性通过 |
| actionlint 1.7.12 / git diff --check | 通过 |
| 历史 APK apksigner 与 aapt2 | 通过，公开证书输出和 JSON 已保存 |
| 本次 Windows 原生 cargo test、EXE、安装 smoke | 待 GitHub Hosted Windows 实际执行，不计通过 |
| 本次 Android ARM64 APK、原签名覆盖升级 APK | 待 GitHub Hosted Android 实际执行及签名条件确认，不计通过 |
| 同 SHA 双端包 / SHA256SUMS / Pre-release | 尚未具备，不生成虚构安装包、哈希或下载链接 |

## 发布门槛与待办

目标继续为未发布的 0.0.2/1002。没有创建 Tag、Release 或占用下一版本；不覆盖历史版本。CI unsigned-ci 产物是未签名编译检查包，不能安装或当作连续升级包，也没有另生成独立签名测试 APK。只有原签名 APK、Windows installer smoke、必要自动化检查和同 SHA 证据全部通过才允许发布。

当前唯一确认的远程访问阻碍是 api.github.com CONNECT 403。域名已加入环境草稿，需在环境设置保存以应用运行时网络配置，然后重试读取失败日志、列出四项 Secrets 的名称、检查本次 CI、触发固定 main SHA 的 release.yml。Secrets 若确实缺失，必须通过仓库安全设置恢复匹配原证书的旧 keystore，不在聊天中索取值。

真机未完成项目仍见 [PHASE2_ACCEPTANCE](PHASE2_ACCEPTANCE.md)：双端安装/原签名覆盖升级、旧 SQLite/人格/聊天/长期记忆保留、系统凭据与真实 API、家庭网线 PC + Wi-Fi Android 发现/双方确认/同步、断网重连/离线补传/冲突/删除/撤销及 Android 生命周期。云 loopback 和 UI Mock 不计入实机通过。
没有修改业务数据库、DDL、聊天/人格/记忆数据或阶段 3 功能。远程提交与 Actions 结果以最终交付中的实际链接为准。
