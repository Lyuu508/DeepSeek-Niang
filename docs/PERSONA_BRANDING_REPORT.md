# dev.2 永久聊天、人格与品牌交付报告

日期：2026-11-12。本地版本 **0.1.0-dev.2**，继承 `/workspace/output/deepseek-ai-companion` 的 dev.1 原工程。未重新创建项目，未进行 GitHub push/PR/tag/Release 或自动发布。

## 实际完成

- 默认仅一个 DeepSeek娘人格，每人格一个稳定永久主空间。新增人格自动建独立设定、主空间和关系/情绪/生活/兴趣容器；切换/改名不创建新会话；最后选择保存在本机 SQLite，重启恢复。
- 前端移除传统新建/最近/默认会话入口。桌面左栏人格切换/管理、中间永久时间线、右栏资料/状态/记忆/用量；手机顶部头像名称、抽屉切换、底部输入。蓝白/深色与中文/IME交互保留。
- 人格配置、有限近期上下文、手动记忆/检索、Token统计隔离。运行中的请求从原会话确定owner，切换不会写到新对象；慢历史快照不能覆盖流式新状态。保留停止、最后一轮重新生成、历史回复变体、错误和中断恢复。
- SQLite v3 以归属映射合并旧会话视图，保留原消息/状态/角色/时间/UUID/用量/同步信封和检查点；升级前一致备份、整事务校验与失败回滚。详见 [迁移说明](PERSONA_MIGRATION.md)。
- Windows productName/窗口标题、HTML/内部品牌和 Android 资源名称统一为 DeepSeek娘。identifier/applicationId 仍为 `io.local.deepseekcompanion`，原生库、私有目录与凭据命名空间不变，未更换签名。
- 阶段2～9八份独立实施/接口/迁移/费用/验收计划已写入 [phases](../DEVELOPMENT_PLAN.md)，没有开始后续完整功能。

## 品牌资源的实际状态

用户指定海滩图在聊天可见，但附件目录目前只收到指令文本，没有可读取的原始 PNG。本次没有伪造 master、重绘角色、从屏幕截图代替原图或声称新图标已生成。UI 用“原图待导入”文字，不使用DS字母头像。

`assets/branding/manifest.json` 真实状态为 pending_source。已完成 `prepare_branding.py` 和 `check-branding.mjs`：保留master原始字节、生成icon/avatar/splash、Windows多尺寸ICO/PNG、五密度Android launcher/adaptive/启动/白色通知蒙版，记录每文件SHA-256，并复制到Tauri/public/Android工程。资源缩放测试使用临时纯色夹具，绝不是正式人物素材。

| 资源/位置 | 本次状态 |
|---|---|
| 原始 master、icon、avatar、splash | 等待同一原始PNG，未生成 |
| Windows标题栏/任务栏/安装器/快捷方式ICO | 接线/脚本完成，正式替换未完成；历史dev.1资源仍保留 |
| Android launcher/adaptive/各密度/启动/通知 | 接线/脚本完成，正式替换/遮罩实测未完成 |
| Windows/Android显示名称 | 配置与Android生成工程字符串已改，尚无本版安装验证 |

原生 Tauri dev/build 和本机构建脚本先校验品牌；缺原图即明确停止，不能沿用旧DS图标打包新版。普通前端预览/共享核心测试继续可用。收到原图后需目视检查头像裁切、脸/眨眼/V手势和各遮罩，再构建，不以算法测试代替真实素材验收。

## 数据与功能边界

环境未发现用户业务数据库；v2升级、备份、回滚和重启通过真实SQLite文件上的合成旧库验证，不能称已迁移用户电脑/手机。实际设备首次启动会在原私有目录备份再迁移，失败不能删库掩盖问题。

API/Token/安全存储原功能保留；本机所有人格共用原本机DeepSeek凭据，用量分别统计，其他设备仍各自配置。没有要求用户把Key发到聊天，没有执行真实DeepSeek调用。当前只有Token统计，模块每日费用/预算是后续计划，不把未知价格当免费。

关系/情绪/生活/兴趣容器按人格独立，但自动引擎未启用，显示“尚未评估/尚未记录”。阶段0 profile级原型及唯一键仍保留，未来相应阶段需要扩展事件/唯一键和任务API，不能声称已实现自主生活。一次一个聊天请求；大规模时间线分页/虚拟列表仍待后续优化，发送上下文始终有上限。

## 测试与构建

| 检查 | 结果 | 能证明的范围 |
|---|---|---|
| Rust workspace离线锁定测试 | 57通过 | 原48+9项人格/迁移/隔离保护；含原21项核心测试 |
| Clippy / rustfmt | 通过 | 共享代码静态检查/格式 |
| TypeScript / Vite生产构建 | 通过 | 共享前端编译，生产JS不含Mock夹具实现 |
| 浏览器UI | 16项Mock通过 | 桌面/390px手机、隔离/流式/停止/重生成/慢读取，无真实API/SQLite/OS凭据 |
| Python构建/导出/图标策略 | 6通过 | 原3+3项原图字节/ICO/adaptive/哈希门槛/Android接线夹具 |
| SQLite DDL | 通过 | 33原要求表+4聊天表+5人格表、13归属列、外键/归属/边界/中文FTS/完整性 |
| Android ARM64原生应用cargo check | 通过 | Tauri IPC、共享核心/聊天、Keystore Rust适配可编译；不是APK或设备测试 |
| Windows凭据适配cargo check | 通过 | Windows目标下SecretStore适配编译；未调用Credential Manager |
| Windows完整应用/EXE/MSI | 未生成 | 无Windows本机，且正式PNG缺失导致品牌门槛拒绝打包 |
| Android新版Gradle/APK | 未生成 | 已有SDK/NDK/JDK可用，但正式PNG缺失，未绕过门槛打包旧图标 |
| 双端安装/升级/运行/真实API | 未执行 | 由用户在设备本地配置Key验收，不能以编译代替 |

本次没有新增外部依赖版本、自动安装新程序、大型模型或使用新付费服务。已有工具链：Rust1.99.1、Node24.22、Pillow13.0.0；旧dev.1 Android已验证构建环境为JDK21/SDK36/NDKr28c/Gradle9.6.1/AGP9.3.1/Kotlin2.2.10。原始DDL0001/0002、原21项测试、relationship/task/video模块逐字节核对保留。

原始命令/日志及明确Mock截图在 [validation-persona](validation-persona/README.md)。旧dev.1安装包和历史报告仍保留，仅代表旧版，不包含新永久人格空间。

已实际执行 Windows Tauri `build --target x86_64-pc-windows-msvc --bundles nsis` 和 Android `android build --apk --debug --target aarch64`，两者均命中 beforeBuildCommand 的原始PNG门槛并以1退出。没有声称构建成功，也没有用旧target中的文件作为本次产物。

## 源码与安装测试

完整文件清单见 [FILES](FILES.md)；源码ZIP由 `python3 scripts/export_source.py` 导出，含逐文件SHA-256，不含数据库、备份、密钥、签名私钥、模型、依赖/构建缓存或安装包。原图缺失状态写入ZIP，不虚构素材。实际下载方式见聊天交付与 [SOURCE_EXPORT](SOURCE_EXPORT.md)。

Google Drive源码上传被自动审批拒绝，原因是原授权仅覆盖安装器/校验文件，未覆盖源码；上传没有发生。源码包已在本地生成并核验，远程私有下载链接等待用户对本次源码明确授权，不通过其他渠道绕过拒绝。

补齐同一原始PNG → `prepare_branding.py --source ...` → `check-branding.mjs` → 目视检查 → Windows/Android构建脚本 → 校验产物/签名 → 对应设备安装。使用 [Windows](INSTALL_WINDOWS.md)、[Android](INSTALL_ANDROID.md) 和 [BUILDING](BUILDING.md) 逐项验证永久时间线、人格切换、数据重启、安全存储和真实流式。

Windows改名可能影响NSIS卸载项/安装目录，实际覆盖升级尚未验证；Android必须沿用原签名，不能卸载清数据或改applicationId冒充升级成功。

**GitHub 未更新，本次修改仅保留在本地。**
