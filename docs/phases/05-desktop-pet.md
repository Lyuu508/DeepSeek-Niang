# 阶段 5：Windows 桌宠

状态：独立计划，当前没有透明桌宠窗口。用户海滩 PNG 仅用于静态品牌资源；不可将单张原图称为已完成 Live2D 动画素材。

## 窗口与共享状态

在现有 Tauri 2 应用增加单独 pet 窗口：透明、无边框、置顶可关、拖动、缩放、隐藏/恢复、托盘、可选开机启动。主窗口、pet 和未来 NAS 共用同一 Chat/Persona/Activity 服务；桌宠不保存另一份人格或自行创建聊天会话。点击打开当前人格永久时间线；双击、快捷聊天、气泡和通知须防重复触发。

```ts
interface PetRenderer {
  load(assetId: string): Promise<void>;
  setCharacter(personaId: string): Promise<void>;
  render(state: { mood: string | null; activity: string | null; speaking: boolean; stale: boolean }): void;
  play(action: 'idle' | 'blink' | 'happy' | 'shy' | 'thinking' | 'angry' | 'sleep' | 'read' | 'video' | 'talk' | 'sing'): Promise<void>;
  setVisemes(events: { atMs: number; value: number }[]): void;
  dispose(): void;
}
interface PetWindowService {
  show(): Promise<void>; hide(): Promise<void>;
  setScale(scale: number): Promise<void>;
  setAlwaysOnTop(enabled: boolean): Promise<void>;
}
```

当前人格选择为本设备偏好；切换时宠物订阅同一 persona_id。动作状态来自未过期情绪/真实活动/已标注虚拟事件，未知状态用待机，不自行声称看视频。生成中可以显示思考/说话状态，但无语音播放时不伪造音频口型；阶段 7 的实际音频/viseme 事件驱动嘴型，唱歌使用对应时间轴。

## 资源和第三方审查

候选 C2006L/dafeiyu、c-dom/deepseek-pet、Andersen216/dsh-whale-girl-live2d 只是研究对象，沿用阶段 0 固定版本初审。重新核对源代码许可与人物、动作、声音分别授权；Live2D Cubism SDK/运行库分发许可也单独审查。不得因仓库公开就复用人物模型，不执行未知 EXE、远程 PowerShell、管理员安装或关闭杀毒要求。

优先复用可许可的渲染/交互机制；渲染器适配层支持 Live2D、授权序列帧/轻量动画。没有动作素材时明确缺项，不用固定 GIF 充当情绪联动。原始海滩 PNG 字节和品牌派生文件仍按 assets/branding 的哈希保存，不重绘角色或混入候选仓库的未知素材。

审查清单：依赖锁、npm 生命周期、Rust build.rs、下载域名、网络请求、系统文件访问、注册表/启动项、自动更新与 GitHub Actions。静态分析和隔离测试只能给出检查范围，不能证明绝对安全。

## 性能、权限与验收

隐藏/屏幕关闭时暂停动画，空闲降帧；设置帧率/纹理缓存上限，记录普通电脑 CPU/内存/GPU 和电量。窗口位置按屏幕工作区保存，热插拔/多屏/DPI 改变后可找回，透明点击区域不干扰正常桌面。只向 pet 窗口开放所需 IPC，不允许通用 shell/文件系统权限或直接读取 Key。

验收：Windows 10/11 实机透明度、缩放、拖动、置顶开关、DPI/多屏、托盘找回、输入焦点、开机启动明确同意、气泡打开永久空间、人格切换不串线、状态过期标记、情绪/活动/语音动作联动、退出进程后无常驻任务。Android 不强制悬浮窗；若未来增加，需单独授权系统覆盖权限。
