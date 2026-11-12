# 阶段 1 源码保存与取回

源码目录：`/workspace/output/deepseek-ai-companion/`。
阶段 0 原始 ZIP 保留，不覆盖。当前已发布版本 `0.0.1`，旧dev.2 ZIP保留不覆盖。

在项目根目录执行：

```bash
python3 scripts/export_source.py
```

默认生成相邻目录的 `DeepSeek-Companion-stage1-v0.0.1.zip` 和 `.zip.sha256`。
可用 `--output <目标ZIP>` 指定可写位置。ZIP 顶层为 deepseek-ai-companion，含所有源码、锁文件、平台插件、构建脚本、assets品牌清单、阶段2～9八份计划和验证文档，另有逐文件大小/哈希的 SOURCE-MANIFEST.json。ZIP CRC 会被检查。

导出使用源码目录白名单，排除 node_modules/target/dist/gen/.git、数据库及 WAL/锁文件、私钥、备份、缓存、日志、安装包和环境密钥文件；`.env.example` 只有说明。凭据形状扫描是一项有限检查，不能证明任意人工写入的敏感内容都能被发现。测试记录中的所有消息均为明确测试数据。

## 如何从当前环境取回

本次已获得明确的公开GitHub发布授权。优先到 [v0.0.1发布页](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.1) 选择 Source code (zip) 或 (tar.gz)，取得两个安装包对应的8ae10e7源码及正式资源。最新main还包含发布后补充的测试说明。以下方法适用于工作区本地额外导出，内部路径仍不等于可下载附件。

优先使用运行此工作区的客户端文件面板：定位 `/workspace/output/`，选择 ZIP 和 SHA-256 文件并使用 Download/下载。若平台只支持下载工作区文件，可将这两个文件复制到该平台指定的可下载文件区；不要上传到 GitHub 或其他公开网盘代替授权。

若工作区有终端和**私有端口转发**，可在项目根目录运行：

```bash
python3 scripts/serve_source.py ../DeepSeek-Companion-stage1-v0.0.1.zip --port 8766
```

它仅绑定工作区自身的 `127.0.0.1:8766`，只提供该 ZIP 的 `/download` 与 `/sha256`，不开放工作区目录。将平台的私有端口转发指向 8766，再使用平台给出的私有地址加 `/download` 取得文件。没有转发时，用户电脑上的 localhost 无法访问云端 localhost。
若平台提供 SSH 私有隧道，可由用户在自己电脑运行 `ssh -L 8766:127.0.0.1:8766 <自己的工作区SSH地址>`，随后浏览器打开本机 `http://127.0.0.1:8766/download`。

不自动建立公网服务器。完成下载后 Ctrl+C 停止服务；若客户端既无文件下载也无私有转发/SSH，必须通过该客户端的导出功能取得文件，本会话无法声称已上传附件。

Windows 校验：`Get-FileHash -Algorithm SHA256 <下载的ZIP>`；Linux/macOS 可用 `sha256sum <ZIP>`（macOS 可用 `shasum -a 256`）。解压后重新安装依赖；安全存储中的 Key 不在 ZIP 内，须在各设备设置。

当前0.0.1源码包含用户提供的原始PNG及69项派生资源，manifest为ready；本版EXE/APK已生成并发布。源码ZIP不是安装器；需要自行构建时先校验资源，再按Windows/Android说明执行。

## dev.2私有交付历史

旧dev.2源码ZIP与SHA-256文件已经用户单独授权，通过Google Drive私有文件完成交付并回读校验；旧文件保留。本次公开GitHub仓库使用用户对0.0.1的另一次明确授权，不会把私有数据库或凭据随源码公开。

## 0.0.1 GitHub交付

[公开仓库](https://github.com/Lyuu508/DeepSeek-Niang) 与 [0.0.1测试版](https://github.com/Lyuu508/DeepSeek-Niang/releases/tag/v0.0.1) 已更新。仅提交源码白名单，数据库/密钥/备份/缓存/旧安装包不入源码树；新EXE/APK作为Release附件发布，公开完整下载/哈希已验证。实际记录见RELEASE_0.0.1.md；后续远程更新仍需用户另行明确授权。
