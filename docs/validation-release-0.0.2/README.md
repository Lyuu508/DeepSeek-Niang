# v0.0.2 发布验证

Release ID445111880，Pre-release已公开，发布Run43571208371成功；Tag与两个包的源SHA均为1c86678963cb83819e7a72cada6d279fcac9736c。13个Release公开附件已在Cloud直接下载HTTP200，逐个比对GitHub资产digest与实际SHA-256，两个安装器同时匹配SHA256SUMS。实际下载APK另用官方Build Tools的apksigner/aapt2复验，签名/包名/版本/ABI与BUILD-RESULTS一致。Android是独立.test开发签名，不能覆盖v0.0.1。

Windows原生测试/安装与窗口启动通过；真实用户设备、家庭网线+Wi-Fi同步、系统凭据/API仍待用户验收。public-download-verification.json仅公开文件名/HTTP状态/大小/哈希，不含临时下载令牌、私钥、密码或用户数据。未来已授权版本直接发布EXE/APK附件，不以Actions ZIP作为唯一交付。
