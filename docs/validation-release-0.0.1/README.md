# 0.0.1 实际发布验证

此目录记录已发布安装包的核验结果。原生Windows截图来自干净的GitHub Windows runner，未输入真实Key或用户数据；不是浏览器Mock截图。

- BUILD-RESULTS.json：实际构建/签名/测试及未验证项。
- published-release.json：公开Release元数据、资源SHA-256及两个构建对应的源码提交。
- public-download-verification.json：四个公开附件完整下载后的长度与SHA-256校验。
- windows-installed-window.png：静默安装后从已安装程序取得的窗口，已目视确认本次正式头像/图标。
- build-tools-review.json：固定官方Actions与本次一次性手动上传步骤的有限静态审查范围。

Windows构建日志保留于 [run 43262321945](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43262321945)，一次性附件转移于 [run 43266632511](https://github.com/Lyuu508/DeepSeek-Niang/actions/runs/43266632511)。两个安装包源码均为8ae10e7c62602fde124989800c6e16d5a951dcb0，tag不随之后的文档补充移动。
