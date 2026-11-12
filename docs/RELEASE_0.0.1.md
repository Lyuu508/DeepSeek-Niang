# DeepSeek娘 0.0.1 测试版

更新：2026-11-13。用户已明确授权新建独立公开仓库 `Lyuu508/DeepSeek-Niang`、上传当前修改代码并提供 Windows EXE 和 Android APK 下载。本次授权不扩展为以后自动更新或自动发布。

## 当前准备状态

- 公开目标仓库已由用户创建：[Lyuu508/DeepSeek-Niang](https://github.com/Lyuu508/DeepSeek-Niang)。本次上传完整0.0.1源码，并保留初始化README提交及项目愿景。此前创建仓库接口权限问题已由用户创建仓库解决。
- 版本：0.0.1；计划标签v0.0.1，GitHub Release标记为测试版/prerelease。尚未创建完整可下载安装的Release。
- 继承原dev.2全部人格/永久时间线与阶段0/1代码，SQLite user_version仍为3；没有重建项目或重置数据。
- Windows/Android显示名DeepSeek娘，identifier/applicationId仍为io.local.deepseekcompanion，凭据命名空间和签名身份不变。
- Android内部versionCode为1001，比旧dev.1实际APK的1000高；显示版本改为0.0.1不会以默认版本码1触发覆盖安装降级。实机升级仍未验证，必须使用同一签名，不卸载清数据。
- 先前指定的海滩图已于11月13日在聊天中再次确认，但消息接口仍只提供内嵌预览，没有可读取的原始PNG文件或下载ID。品牌校验继续有效，不能使用旧DS资源或临时测试图发布新包。

## 安装包出口

Windows EXE/APK都必须从本次0.0.1源码构建，保存代码commit、工具链、产物哈希和构建日志；不把旧0.1.0-dev.1文件改名上传。Windows可由实际Windows环境或经验证的交叉流程构建，Android复用已有SDK36/NDKr28c/JDK21环境。

收到同一原始PNG后：

```powershell
python scripts/prepare_branding.py --source "C:\你的目录\原图.png"
node scripts/check-branding.mjs
.\scripts\build-windows.ps1
# 配好Android SDK/NDK/JDK后：
.\scripts\build-android.ps1
```

Linux的Android入口为 `sh scripts/build-android.sh`。构建组件、签名/覆盖安装和设备验证见 [BUILDING](BUILDING.md)、[Windows](INSTALL_WINDOWS.md)、[Android](INSTALL_ANDROID.md)。原始PNG缺失时构建应停止，不能改掉门槛冒充完成。

## 发布与验证

本地先复核锁文件、源码隐私排除、核心测试、前端、DDL和原生共享检查，再上传用户指定仓库。公开仓库不含聊天数据库、备份、API Key、账号凭据、配对私钥、Android签名私钥、模型、依赖缓存或未授权人物素材。

完整测试Release需包含：本版Windows安装EXE、ARM64 APK、源码ZIP、SHA-256清单，以及清楚标注构建/签名/运行测试范围的说明。只有真实产物存在且核验后才添加下载链接；未测试设备运行和真实DeepSeek必须标注。真实Key由用户在设备本地设置，不发到聊天。

本次0.0.1复验：57项Rust（含原21项）、16项UI Mock、6项策略测试、TypeScript/Vite、Clippy/fmt、SQLite及Android原生Rust检查全部通过。Windows和Android打包入口均实际执行，因原始PNG缺失在品牌校验处以1退出，没有新版EXE/APK。运行/真实API未验证。原始记录见 [validation-release-0.0.1](validation-release-0.0.1/summary.json)。

修改范围为12处本项目版本/锁文件元数据、Android内部版本码、发布约束与说明和本次验证记录。没有升级外部依赖、删除旧迁移/模块或将私有签名加入源码。

阶段2～9仍只有计划，未开始实现。

## 安装包待补的输入

源码上传已获明确授权。安装包仍需可读取的原始PNG字节：把同一张原PNG压成ZIP作为文件附件，或直接放入仓库 `assets/branding/deepseek-niang-master.png`。工具不能从聊天预览重建一个文件并称为原图。

原图可读取后，生成/目视检查正式图标，再从同一版本源码真实构建EXE/APK，核验产物后发布测试Release及下载附件。本次不将没有安装包的源码上传称为安装包发布完成，原有0.1.0-dev.1包也不作为本版附件。

授权仍限本次0.0.1测试版，不需重复确认同一发布范围。双端实机与真实API仍由用户在设备本地配置密钥验收。
