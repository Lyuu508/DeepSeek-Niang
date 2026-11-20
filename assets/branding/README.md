# DeepSeek娘品牌资源

原始 PNG 由用户提供，尺寸 1254×1254，1724316 字节。master 与上传文件逐字节一致，SHA-256 为 `070adedf4a8896745ea8fb7755b3cbe4b673bcb51d1edc88cc76e46ea937aff1`。清单记录 70 项派生资源。

`manifest.json`记录原图和每个派生文件哈希，以下资源已生成：

| 目标 | 用途 |
|---|---|
| deepseek-niang-master.png | 原始 PNG，字节不变，记录 SHA-256 |
| deepseek-niang-icon.png | 完整海滩构图，1024px |
| deepseek-niang-avatar.png | 裁切突出脸部、眨眼、大眼睛及手势，512px |
| deepseek-niang-splash.png | 完整构图启动页，1024px |
| ../../apps/companion/src-tauri/icons/*.png、icon.ico | 窗口、任务栏、快捷方式及安装器，多尺寸 |
| android/ | mdpi～xxxhdpi launcher、adaptive foreground/background、启动画面和白色通知蒙版 |

如需重新生成，使用已审核的本地 Pillow，仅裁切、缩放和制作平台要求的透明/单色蒙版：

```powershell
python scripts/prepare_branding.py --source "C:\你的目录\原图.png"
node scripts/check-branding.mjs
```

脚本不联网、不运行模型、不安装依赖。缺少 Pillow 时先由用户确认安装；本轮环境已有 Pillow 13.0.0。原始图不同于已保存 master 时脚本停止，避免覆盖唯一原图。

Adaptive Icon 的中心 72dp 可见区由原图覆盖。108dp 图层余量延伸原图边缘像素。圆形和圆角方形预览取自实际图层，保留头饰、眼睛和比耶手势。启动画面另用 46dp 安全区，通知图标使用白色 alpha 蒙版。[遮罩预览](android-mask-preview.png)

生成后保留 master 哈希与所有派生文件哈希，复制头像/启动图至前端 public，原生图标至 Tauri icons；Android 准备脚本将资源复制到生成工程并设置主题。`beforeBuildCommand` 及本机构建脚本运行校验门槛，原图缺失时明确停止原生安装包构建。普通前端预览继续可用，以“原图待导入”文字标识，不使用 DS 字母占位。

用户指定该图作为本项目的统一形象；本工程不将其描述为 DeepSeek 公司官方授权商标。未来对外发行仍应记录原作者和实际许可依据。
