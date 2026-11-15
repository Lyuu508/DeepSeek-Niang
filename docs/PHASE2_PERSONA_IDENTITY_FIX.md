# 阶段 2：扫码重复默认人格修复

## 根因与接管事实

接管 GitHub 最新 main `3704780c59ada319282a2809e7ba93876fdb720f`，原版本 0.0.2 / 1002。阶段 0/1 的工程、聊天、人格与原始图标已经存在；阶段 2 已有真实 TLS 同步、单按钮二维码/相机与持久测试签名，不重新开发这些功能。接管基线 CI 43698512920 的 Windows/Android 构建成功，浏览器横屏导航回归失败；本版修复该布局竞争。

两端首次安装都会由 `ensure_default` 使用各自 profile UUID 自动创建默认人格。旧配对成功后，增量同步保留双方人格集合，缺少“Android 初始默认人格归属 Windows 主机”的绑定，因此显示两个同名默认项。二维码生成/扫描本身没有直接调用新增人格，错误出在初始化身份与配对后数据集汇合规则。

当前没有用户手机、实际用户数据库或设备通道，所以不能报告用户设备的重复行数，也不能确认任意同名人格的来源。自动处理只针对数据库明确标识的本机自动默认人格；手动创建记录即使同名也不合并。

## 修复规则

1. 二维码仍只携带短期连接和安全配对信息，保留两分钟过期、一次性令牌、私钥证明、TLS 1.3/mTLS、证书固定与撤销检查。
2. Windows 在已验证的 TLS 回复中提供 `HostAuthority {device_id, profile_id, persona_id}`。手机核对主机与受信证书身份后写本机 `lan_host_binding`，不从二维码创建人格。不允许无提示地切换已绑定主机。
3. 手机收到实际主机档案、设备和人格同步行后，才将本机 `is_builtin=1` 的初始人格关联到主机原 ID。主机人格的名称、头像、人设、System Prompt、模型配置和设置作为首配初始权威。既有配对在两端升级后也通过已认证 Exchange 采用主机权威，无需再次扫码。
4. `persona_links` 保存原 ID → 主机 ID、来源设备/主机/原因/时间。列表与选中项使用 canonical ID；旧消息、记忆、用量、关系事件按别名范围聚合，新写入使用主人格。不改变原始消息 ID、记忆 ID、来源或旧配置行。
5. 真正手动人格不改名、不删除、不归并；同一手机重复扫码、重放同步、断线恢复不会增加第二条关联或设备身份。配对协议本来就有证书设备 ID、可信 peer、游标与逻辑修订，本版没有另建重复配对数据库。
6. 后续资料/消息/记忆/关系/情绪/生活按同一个 persona 范围与已签名增量事件同步。逻辑版本和事件 UUID 处理并发，冲突副本保留，删除墓碑防复活；墙钟不决定覆盖次序。

## 数据库与回滚

新增 `0005_pairing_persona_identity.sql`、`0006_companion_foundation.sql`，当前 schema 6。0001～0004 字节与已发布版本一致。Online Backup 在迁移前生成 `.pre-v6-UUID.bak`，包含 WAL 一致状态；全部待执行 DDL 同一事务，外键/行数/完整性校验后提交，失败回滚。真实 SQLite 旧 v4 合成库及注入 DDL 失败测试已执行。

这里的安全关联是可审计的逻辑迁移，没有 `DELETE persona` 或重写用户消息来掩盖重复。合成旧库验证备份与自动回滚，不代表已在主人手机迁移。成功升级后若已产生新聊天，直接恢复旧备份会缺少升级后记录，不能擅自降级、卸载、删库或手动删关联。需关闭应用，保全当前库与全部备份，再在受控副本中恢复核对；未提供无损一键“降级并保留新写入”功能。

## 自动化验收

优先修复阶段通过 81 项 Rust 和独立 SQLite 检查后，才开始阶段 3。最终整个 workspace 本地 103 项 Rust 通过，原测试未删除或忽略；CI 会在同一源码重新执行 Linux/Windows 测试。测试使用独立真实 SQLite 数据库、实际回环监听端口与 rustls TLS 握手，不是 Mock TLS 成功。

| 要求 | 自动化证据 |
|---|---|
| 主机已有一个人格，扫码仍显示一个 | `qr_pairing_uses_one_host_persona_and_preserves_ids_across_repeat_scan_and_offline_resume` |
| ID、名称、头像、System Prompt 相同 | 同上，比较原主机 UUID、头像资源、人设和完整设置 |
| 好感、记忆一致 | 同上及 `real_tls_sync_converges_bond_mood_memory_preferences_and_virtual_life` |
| 手机消息到电脑、电脑消息到手机 | 同上，真实双向 TLS Exchange |
| 重复扫码不新增 | 同上，固定设备与 canonical ID/行数 |
| 断线恢复无重复、原消息 ID/历史保留 | 同上及 `paired_canonical_persona_survives_actual_host_disconnect_and_restart` |
| 手动同名人格保留 | `pairing_never_merges_user_created_same_name_personas` |
| 既有配对升级自动采用主机 ID | `existing_paired_android_adopts_authenticated_windows_authority_without_new_persona` |
| 备份、原行和失败回滚 | `v4_upgrade_is_backed_up_preserves_rows_and_rolls_back_on_failure` |

这是真实独立数据库与本机网络自动化，不是家庭 Windows + Android 实机验收。手机相机、权限、系统栏、家庭路由器网线+Wi-Fi、真实用户重复库与长期离线仍由主人按 PHASE3_ACCEPTANCE.md 检查。

主要改动：core `pairing.rs/persona.rs/chat.rs/replication.rs/storage.rs`，sync `lib.rs/tests.rs`，native `sync.rs/lib.rs`，contracts 头像/状态字段，新迁移与配套测试。完整源码清单以 v0.0.3 提交 diff 为准。
