# dev.2 本次文件清单

本页保留dev.2本地交付时的208文件清单与当时状态。当前0.0.1已收到原始PNG、生成正式资源并在GitHub发布EXE/APK；新增构建/验证文件及当前结果见 [RELEASE_0.0.1.md](RELEASE_0.0.1.md)。以下历史缺图/未发布描述不代表当前状态；完整当前清单以Git及SOURCE-MANIFEST为准。

相对已交付 dev.1 安装测试源码包：修改 46、新增 45、删除 0，原样保留 117；当前源码文件 208 个（另附 ZIP 内 SOURCE-MANIFEST.json）。

旧迁移0001/0002、原21项测试、关系/任务/视频源文件按字节保留。完整逐文件哈希以ZIP的SOURCE-MANIFEST为准；机器可读列表见 [CHANGED_FILES.json](CHANGED_FILES.json)。

主要模块：persona.rs 与 v3迁移负责身份/主空间/隔离，chat/storage/Service负责兼容旧时间线与请求归属；React的Avatar/PersonaManager/main负责人物导航及安全切换；品牌脚本严格等待原始PNG；八份阶段文档只规划后续功能。

## 修改文件

```text
.env.example
AGENTS.md
CHANGELOG.md
Cargo.lock
Cargo.toml
DEVELOPMENT_PLAN.md
README.md
SECURITY.md
apps/companion/index.html
apps/companion/package.json
apps/companion/src-tauri/Cargo.lock
apps/companion/src-tauri/Cargo.toml
apps/companion/src-tauri/src/lib.rs
apps/companion/src-tauri/tauri.conf.json
apps/companion/src/defaults.ts
apps/companion/src/main.tsx
apps/companion/src/native.ts
apps/companion/src/style.css
apps/companion/src/test-fixture.ts
crates/companion-chat/src/service.rs
crates/companion-core/src/chat.rs
crates/companion-core/src/chat/tests.rs
crates/companion-core/src/event.rs
crates/companion-core/src/lib.rs
crates/companion-core/src/storage.rs
crates/companion-secrets/src/windows.rs
docs/BUILDING.md
docs/DATABASE.md
docs/INSTALL_ANDROID.md
docs/INSTALL_WINDOWS.md
docs/SOURCE_EXPORT.md
docs/THIRD_PARTY_REVIEW.md
package-lock.json
package.json
packages/contracts/package.json
packages/contracts/src/index.ts
plugins/companion-keystore/Cargo.lock
plugins/companion-keystore/Cargo.toml
scripts/build-android.ps1
scripts/build-android.sh
scripts/build-windows.ps1
scripts/check_schema.py
scripts/export_source.py
scripts/prepare_android.py
tests/browser/chat.cjs
tests/test_build_policy.py
```

## 新增文件

```text
apps/companion/src/Avatar.tsx
apps/companion/src/PersonaManager.tsx
apps/companion/src/branding.ts
assets/branding/README.md
assets/branding/manifest.json
crates/companion-core/src/persona.rs
crates/companion-core/src/persona/tests.rs
db/migrations/0003_personas.sql
docs/CHANGED_FILES.json
docs/PERSONA_BRANDING_REPORT.md
docs/PERSONA_MIGRATION.md
docs/phases/02-lan-sync.md
docs/phases/03-relationship-memory.md
docs/phases/04-life-proactive.md
docs/phases/05-desktop-pet.md
docs/phases/06-video-understanding.md
docs/phases/07-voice-singing.md
docs/phases/08-nas-migration.md
docs/phases/09-release-optimization.md
docs/phases/PROVIDER_CONTRACTS.md
docs/validation-persona/README.md
docs/validation-persona/android-packaging-attempt.txt
docs/validation-persona/android-rust-check.txt
docs/validation-persona/baseline-preservation.json
docs/validation-persona/browser.txt
docs/validation-persona/clippy.txt
docs/validation-persona/dependency-preservation.json
docs/validation-persona/frontend-build.txt
docs/validation-persona/packaging-gate.txt
docs/validation-persona/persona-desktop-mock.png
docs/validation-persona/persona-mobile-mock.png
docs/validation-persona/persona-welcome-mock.png
docs/validation-persona/policy-tests.txt
docs/validation-persona/results.json
docs/validation-persona/rust-tests.txt
docs/validation-persona/schema.json
docs/validation-persona/summary.json
docs/validation-persona/typecheck.txt
docs/validation-persona/windows-packaging-attempt.txt
docs/validation-persona/windows-vault-check.txt
scripts/check-branding.mjs
scripts/prepare_branding.py
tests/browser/archive/README.md
tests/browser/archive/stage1-chat.cjs
tests/test_branding.py
```

## 交付边界

本次不含用户业务数据库、备份、API Key、签名私钥、依赖/构建缓存、模型或安装包。原始PNG未收到，不能把旧DS资源或测试纯色图当作新素材。源码ZIP不是安装器；Windows/Android本版打包都因原图门槛中止，未实机验收。

**GitHub 未更新，本次修改仅保留在本地。**
