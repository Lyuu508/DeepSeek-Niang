# DeepSeek娘品牌资源

当前状态：**工具仍待取得原始PNG文件字节**。2026-11-13用户再次发送了同一张海滩图，聊天中可见，但消息接口未给工具可读取的PNG路径或下载ID。请通过ZIP文件附件或仓库的 `assets/branding/deepseek-niang-master.png` 提供原图；不能从预览重绘或用旧DS资源冒充。

`manifest.json` 的 `pending_source` 是实际状态；以下目标文件尚未生成：

| 目标 | 用途 |
|---|---|
| deepseek-niang-master.png | 原始 PNG，字节不变，记录 SHA-256 |
| deepseek-niang-icon.png | 完整海滩构图，1024px |
| deepseek-niang-avatar.png | 裁切突出脸部、眨眼、大眼睛及手势，512px |
| deepseek-niang-splash.png | 完整构图启动页，1024px |
| ../../apps/companion/src-tauri/icons/*.png、icon.ico | 窗口、任务栏、快捷方式及安装器，多尺寸 |
| android/ | mdpi～xxxhdpi launcher、adaptive foreground/background、启动画面和白色通知蒙版 |

收到原图后，使用已审核的本地 Pillow，仅裁切、缩放和制作平台要求的透明/单色蒙版：

```powershell
python scripts/prepare_branding.py --source "C:\你的目录\原图.png"
node scripts/check-branding.mjs
```

脚本不联网、不运行模型、不安装依赖。缺少 Pillow 时先由用户确认安装；本轮环境已有 Pillow 13.0.0。原始图不同于已保存 master 时脚本停止，避免覆盖唯一原图。

Adaptive Icon 把完整图缩放到 108dp 画布中心的 46dp 方框，其对角线小于 66dp 圆形安全区；外部背景取原图天空像素。正式验收仍需检查实际裁切和圆/圆角方形 launcher 预览，必要时只调整裁切或缩放参数。通知图标按 Android 要求从原图明暗提取白色 alpha 蒙版，不作为全彩角色图替代。

生成后保留 master 哈希与所有派生文件哈希，复制头像/启动图至前端 public，原生图标至 Tauri icons；Android 准备脚本将资源复制到生成工程并设置主题。`beforeBuildCommand` 及本机构建脚本运行校验门槛，原图缺失时明确停止原生安装包构建。普通前端预览继续可用，以“原图待导入”文字标识，不使用 DS 字母占位。

用户指定该图作为本项目的统一形象；本工程不将其描述为 DeepSeek 公司官方授权商标。未来对外发行仍应记录原作者和实际许可依据。
