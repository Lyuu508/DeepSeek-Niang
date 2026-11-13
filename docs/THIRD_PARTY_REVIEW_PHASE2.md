# 阶段 2 依赖与构建来源检查

基线依赖版本不整体升级。新增直接依赖锁定并进入 Cargo.lock / npm package-lock：

| 依赖 | 版本 | 作用与边界 |
|---|---|---|
| ring | 0.17.14（复用） | CSPRNG 与 Ed25519，调用成熟密码实现 |
| rustls | 0.23.46（复用） | 生产会话仅允许 TLS 1.3；WebPKI 与双方证书验证，无跳过校验器 |
| rcgen | 0.14.10 | 生成本机自签设备证书，不生成 Android 应用签名 keystore |
| serde_jcs | 0.2.0 | RFC 8785 规范化签名输入 |
| mdns-sd | 0.21.7 | Windows 私有局域网 mDNS，候选地址不等于可信身份 |
| qrcode | 0.14.1 | Rust 二维码编码，短期 token 不写数据库/日志 |
| jsqr | 1.4.0 | 本地图片二维码解码，不访问外部识别服务 |
| x509-parser | 0.18.1（复用已锁版本） | 提取公钥用于首配 TLS 私钥持有证明，不手写证书解析 |
| subtle | 2.6.1（复用） | 常量时间 token 摘要比较 |

npm 使用 --ignore-scripts，Rust/下载包保留官方校验与 TLS；没有执行第三方角色/模型仓库脚本，没有下载大型模型或调用新付费服务。新 crates 的传递依赖由锁文件保留，静态检查不构成完整供应链安全审计。

官方 Actions 固定完整 SHA：checkout 3d3c42e5aac5ba805825da76410c181273ba90b1、setup-node 949feb2413d6458794dcd2491c4babbbce0c15c1、upload-artifact cf430e030ddbb5b0abf93d22962f4752f3646cd9 沿用基线；新增 setup-java b6effb05e454b25005698d916606bdc6ffcbf961（官方 tag v5.7.0）与 download-artifact 37930b1c2abaa49bbe596cd826c3c89aef350131（官方 v7.0.0），已核对远程 tag、action.yml 输入及 Node 运行声明。仅最终发布 job 授予 contents:write。

Cloud 使用 Debian 官方签名仓库的 JDK/SQLite/Tauri 包，在工作区解包，不获取 sudo 或修改系统权限。Android command-line tools 使用 dl.google.com 固定 SHA-256 `0877a1d048fe4a24efe2eff536ca4223f7adeb58648bb81909d33c446918cfa8`；SDK 36、NDK r28c 由官方 CLI 下载校验，Gradle 9.6.1 使用官方发布的 SHA-256 `9c0f7faeeb306cb14e4279a3e084ca6b596894089a0638e68a07c945a32c9e14`。

签名材料只经受保护的 Actions Secrets → 临时文件 → 签名工具，成功或失败均清理，不进 Artifacts。构建日志包装器保留底层退出码并过滤绑定凭据，Artifacts 只含包、报告与过滤后的日志。发布前证书指纹、包名、版本、ABI、ZIP 与哈希必须匹配。

Workflow 静态检查使用 rhysd/actionlint 官方 v1.7.7 Linux 发行包，按同版本官方 checksums.txt 核对 SHA-256 后执行；全部工作流的语法与表达式检查通过。静态检查不代替 GitHub Runner 的实际构建。
