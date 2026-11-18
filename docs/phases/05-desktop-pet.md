# 阶段 5 · Windows 桌宠

**v0.0.6 测试版**将桌宠装入 Windows 主客户端。通过该版本的 Windows EXE 安装或更新后，在应用顶部打开「桌宠」。

## 使用

1. 在 Windows 应用顶部打开「桌宠」，勾选「显示桌宠」。全身角色在当前屏幕工作区内横向走动，在边缘转身，走一段后停下休息。拖动角色可换一个位置或屏幕。
2. 鼠标移到角色上会停步并显示状态和按钮。单击角色挥手，双击打开聊天，右键打开菜单。点状态查看活动，点「快聊」可在角色上方发送和停止生成，回复会同步显示在主窗口。展开面板期间暂停走动。
3. 「仅显示桌宠」收起主窗口，双击托盘图标或选择「打开聊天」恢复。主窗口关闭按钮和托盘「退出 DeepSeek娘」都会退出应用进程。
4. 「随当前 Windows 账户登录启动」默认关闭。启用后 Windows 会启动 DeepSeek娘 并恢复桌宠显示偏好。设置属于当前 Windows 账户，可在这里关闭。
5. 右键菜单和主窗口都可暂停自动走动。开启「减少动态效果」后，桌宠停在当前位置并使用静帧。托盘的「找回桌宠位置」将角色放回主屏工作区。

快聊使用应用已配置的 DeepSeek API Key。收到实际回复时，小窗口只显示本次快聊的简短预览，完整记录在聊天页。活动及分享气泡只显示状态和提示，点击后可前往聊天阅读正文。

## 安装与更新

Windows 安装器使用现有的 `io.local.deepseekcompanion` 应用标识与本地数据目录。更新前退出旧程序并备份本地数据。Windows 主程序包含桌宠页面、原生移动逻辑和动画。走动与挥手无需 API Key。

Windows 使用中文 x64 NSIS 安装程序和 Microsoft Edge WebView2 Runtime，测试安装包沿用以前的自签名证书。Android APK 使用 `io.local.deepseekcompanion.test2` 和同一已发布签名，可在受支持的设备覆盖 v0.0.5 test2 安装。桌宠功能在 Windows 提供。

在 Windows 上可从工程根目录运行 `pwsh -NoProfile -File scripts/build-windows.ps1` 构建主客户端。发布流程按同一源码提交构建双端，核对签名后附上安装文件和 SHA-256。

## 实现与边界

- 透明桌宠使用独立 Tauri WebView。Rust 依据单调时钟和 Windows 工作区更新 HWND 的实际屏幕坐标。走动期间保持 Z 序和窗口焦点，遇到距离足够的边界时换向。打开面板、鼠标悬停、拖动和挥手期间暂停位移。朝向和动作通过桌宠专用 IPC Channel 更新。
- Windows 根据角色轮廓及当前控件裁剪窗口。默认隐藏状态和工具栏。位置按所在显示器工作区比例保存在 `application_settings` 的设备作用域；移动时跳过位置写入，静止后保存。显示器不可用时可通过主屏恢复。
- 实际生活进度来自已保存的活动检查点，过期时显示最后记录。情绪和正在生成的回复使用状态标记。隐藏桌宠会销毁窗口并停止移动。应用设置及 Windows 减少动态效果偏好都能停止自动位移和帧动画。
- 快聊经原 `ChatService` 启动，在写入前核对当前人格。主窗口接收同一次生成的流式事件。桌宠应用权限包括快照、动作订阅、窗口操作、快聊和停止快聊。密钥管理、历史读取和同步等指令保留在主窗口。
- 登录启动只在对应开关操作后写入当前用户的 Windows `Run` 项，键名为应用标识，目标为当前可执行文件。桌宠隐藏和应用退出的处理分别由窗口和托盘执行。
- Android 构建入口排除桌宠窗口和动画文件。正式应用的品牌原图、派生图标和数据库迁移沿用现有版本。

## 资源与依赖

三个桌宠动作来自 PC2005-cloud/dsh-pet。读取固定提交中的三段透明视频，核对 Git blob 指纹并检查媒体轨道后，裁剪转码为 WebP。资源遵循 CC BY-NC-SA 4.0，署名和变更明细随源码。移动及拖动代码由本项目实现。上游 Electron 运行时与脚本仅供只读审查。[原项目](https://github.com/PC2005-cloud/dsh-pet) · [素材许可与转换记录](../../apps/companion/src/desktop-pet/media/NOTICE.md)

其他候选素材检查记录保留供后续选型。dafeiyu 的 MIT 许可缺少逐个素材授权，deepseek-pet 的许可明确排除角色素材，dsh-whale-girl-live2d 的模型和 Cubism Core 各有分发条款。[dafeiyu](https://github.com/C2006L/dafeiyu/tree/7ca2bb30f977e01c52198e27cff1e9264c92c755) · [deepseek-pet](https://github.com/c-dom/deepseek-pet/blob/eb2ef68fa648df3f1b5246fff861655a780e6daf/LICENSE) · [Live2D 许可](https://github.com/Andersen216/dsh-whale-girl-live2d/blob/f977bef508690e75ba6535cd8f4ce18e7f6ba9b0/NOTICE.md)

Windows 仅启用 Tauri 已锁定的托盘功能和既有 `winreg`、`windows-sys` 版本。外部 Cargo 包和 npm 包的版本保持 v0.0.5 锁定集合。前端 CSP 及主窗口 API 端点沿用原配置，桌宠拒绝外部导航和弹窗。桌宠渲染无需联网，实际快聊走原 DeepSeek 客户端。

## 本地验证

| 项目 | 结果 |
| --- | --- |
| Rust 共享业务 | 146 项通过，Clippy 通过 |
| 桌宠走动算法 | 3 项通过，含负坐标显示器、边缘转身、交互暂停、系统恢复与短距离限制 |
| 桌宠浏览器夹具 | 7 项通过，含透明角色、走动指令、悬停、拖动、挥手、快捷聊天和减少动态效果 |
| 主窗口浏览器夹具 | 两种安装身份各 32 项通过 |
| 数据与版本 | 24 项 Python 测试、5 项 Node 测试、schema 8、69 项品牌素材检查均通过 |
| Android 前端 | 本地编译通过。发布 APK 的版本、身份和签名由同源发布流程核验 |
| Windows 集成 | 发布流程在 Windows runner 安装并启动主客户端，从安装后的程序核对桌宠入口和三段动画。实机透明点击、多屏和走动仍待验收 |

主要实现位于 `apps/companion/src-tauri/src/desktop_pet/` 和 `apps/companion/src/desktop-pet/`。共享状态与本机偏好位于 `crates/companion-core/src/companion/pet.rs`。浏览器夹具位于 `tests/browser/desktop-pet.cjs`。

## 实机验收

- Windows 10/11 下打开桌宠，移开鼠标后观察角色自动走动、停留与转身。确认经过其他窗口时焦点保持。检查图外空白区域可点击后面的应用，多屏拖动、DPI 切换和拔下副屏后可找回。
- 测试任务栏托盘、尺寸范围、置顶、关闭飘窗、仅显示桌宠和完全退出。明确开启登录启动后重新登录一次，再关闭并确认该启动项移除。
- 切换人格时头像、活动和快聊草稿各自对应。发送后主窗口只有一条用户消息及同一条回复，隐去桌宠不会取消同一次生成。
- 关闭生活任务或制造连接中断后查看过期标签。读取分享后气泡消失，旧活动不会变成真实联网行为。
- 本机观察 idle/生成时 CPU、GPU、内存。开启减少动画及隐藏时检查表现，报告要注明硬件和显示缩放。
