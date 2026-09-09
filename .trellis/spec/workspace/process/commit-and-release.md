# 提交信息与发版流程

> 提交信息决定 `git log` 与 CHANGELOG 的可读性;发版一旦推到 crates.io 不可撤销。两件事都有固定 SOP,不靠记忆。
> 来源:`../rust/index.md`「硬约束速览」第 7、8、13 条、`README.md`「自动化发版与 CI/CD」、`xtask/src/release.rs`、`.github/workflows/release.yml`、`CHANGELOG.md` 顶部约定。

---

## 1. 提交信息(Conventional Commits,中文描述)

格式:`<type>(<scope>): <中文描述>`,一行 ≤ 72 字符;需要展开时空一行写正文,说「为什么」而不是「改了什么」(diff 已经说了)。

| type | 用于 | 例 |
|---|---|---|
| `feat` | 新功能 / 新公开 API / 新子命令 | `feat(core): 支持解析预发布版本号` |
| `fix` | 修 bug | `fix(cli): --verbose 未覆盖 RUST_LOG 为空的情况` |
| `docs` | 只改文档 / 注释 / README / spec | `docs: 补充 crates.io 发布前检查清单` |
| `refactor` | 不改行为的重构 | `refactor(core): 把版本比较拆成独立函数` |
| `test` | 只加 / 改测试 | `test(core): 覆盖空前缀的拒绝路径` |
| `chore` | 构建、依赖、脚手架、发版 | `chore(xtask): 新增 --keep-going`、`chore(deps): 升级 clap 到 4.6` |
| `ci` | `.github/workflows/*` | `ci: msrv job 改用 rust-version 自动读取` |
| `perf` | 性能优化 | `perf(core): 避免解析时的中间 Vec` |

**scope 取法**:crate 名去掉公共前缀后的短名(`my-app-core` → `core`,`my-app-cli` → `cli`);`xtask` 就叫 `xtask`;依赖统一 `deps`;跨多个 crate 或仓库级改动不写 scope(`docs: ...`、`ci: ...`)。

**几条硬规则**:

- 一个提交一件事;机械性大改(格式化、批量重命名)单独一个提交,方便 `git blame` 跳过。
- 破坏性变更在 type 后加 `!`:`feat(core)!: Config::new 改为返回 Result`,并在 CHANGELOG 里以 **BREAKING** 开头。
- 影响使用者的改动,`CHANGELOG.md` 的 `[Unreleased]` 条目**与代码同一提交**(`../rust/dependencies-and-changelog.md` §4)。
- 新增 / 升级依赖、新增 crate,`Cargo.lock` 同一提交;CI 与发版都 `--locked`,锁文件过时直接失败。
- 提交前 `cargo xtask ci` 通过;不要 `--no-verify` 绕过。
- 发版提交由 `cargo xtask release` 自动生成,固定为 `chore(release): vX.Y.Z`(`xtask/src/release.rs:186`),不要手写。
- 提交信息里不 @ 人、不贴长 URL;关联 issue 写在正文末尾 `Refs: #12` / `Fixes: #34`。

---

## 2. 版本号的唯一来源

- 只在根 `Cargo.toml` `[workspace.package].version` 维护;成员全部 `version.workspace = true`;`[workspace.dependencies]` 里内部 crate 的 `version` 由 `cargo xtask release` 同步(`xtask/src/version.rs` `write_workspace_version`)。
- **不要手改**成员 `Cargo.toml` 或 `Cargo.lock` 里的版本;`cargo xtask version check` 会报不一致,`release.yml` 的 `verify` job 也会拦。
- 全部成员统一版本、一起发(lockstep):某个库这次没改也会跟着 bump,这是有意为之,换来内部依赖永远同版本。
- 版本判定见 `../rust/dependencies-and-changelog.md` §4.3(0.x 阶段 minor 即破坏性)。

---

## 3. 发版 SOP

### 3.1 发版前(人做)

- [ ] `main` 分支,本地与 `origin/main` 同步,工作区干净(`git status` 无输出)
- [ ] `CHANGELOG.md` `[Unreleased]` 下有条目,且每条都是「对使用者的影响」;破坏性以 **BREAKING** 开头
- [ ] 确定 bump 级别:只有 `Fixed` → `patch`;有 `Added` / 非破坏 `Changed` / MSRV 提升 → `minor`;有 **BREAKING**(1.0 后)→ `major`
- [ ] `cargo xtask ci` 通过;有库 crate 时 `cargo publish --workspace --dry-run --locked` 通过
- [ ] 首个版本 / 新库 crate:`Cargo.toml` 的 `description` / `keywords` / `categories` / README 的 TODO 已替换(`../../crates/library/crate-anatomy.md` §3);crates.io 上已配置 Trusted Publishing(首次需手动 `cargo publish -p <name>` 一次,`release.yml:180-183`)
- [ ] 有二进制要打包:`release.yml` 顶部 `PKG_NAME` / `BIN_NAME` 已填(`release.yml:21-24`),两者要么都填要么都空

### 3.2 执行(一条命令)

```bash
cargo xtask release minor --dry-run   # 先看计划:检查项全绿、将写入的版本号、将切的 CHANGELOG 段
cargo xtask release minor             # 真正执行;或 patch / major / 具体 x.y.z / x.y.z-beta.1
```

`xtask/src/release.rs` 的顺序(`README.md`「一键发版工作流」逐条对应):

1. 安全检查(`safety_checks`,全部通过才动文件):工作区干净 → 在 `main` 且与 `origin/main` 同步 → 本地与远端都没有同名 tag → 目标版本不低于当前 → `[Unreleased]` 非空(已手动切好段落则跳过)
2. 写版本号:纯文本替换根 `Cargo.toml` 两处(保留注释)→ `cargo update --workspace --offline` 刷新 `Cargo.lock`,失败即中止并提示回滚
3. 切 CHANGELOG:`[Unreleased]` → `## [x.y.z] - 日期`,更新底部比较链接
4. `git commit` 只含 `Cargo.toml` + `Cargo.lock` + `CHANGELOG.md`,信息 `chore(release): vX.Y.Z` → 附注 tag `vX.Y.Z` → `git push --follow-tags`

`--no-push`:本地 commit + tag 后停下,自己检查后 `git push --follow-tags`。

### 3.3 推送 tag 之后(机器做,`release.yml`)

```text
tag v* → verify(版本 / 内部依赖 / CHANGELOG 段落一致;cargo test --locked;导出 Release 说明)
       → build(仅 PKG_NAME/BIN_NAME 非空;五目标矩阵打包 + SHA256SUMS)
       → publish-crates(cargo publish --workspace,Trusted Publishing,按依赖顺序)
       → publish(GitHub Release,说明 = CHANGELOG 段落,附件 = 二进制包)
```

- crates.io 刻意排在二进制构建**之后**:让所有可能失败的编译先跑完,因为 publish 不可撤销。
- 任一环节失败后续不执行。**`publish-crates` 之前失败**:修好后删 tag(`git tag -d vX.Y.Z && git push origin :vX.Y.Z`)重推即可。**`publish-crates` 已成功**:不能重发同版本,只能 bump 一个新版本重走流程;发错内容用 `cargo yank --vers X.Y.Z -p <name>` 标记(不删除,已依赖者仍可构建)。
- `verify` 调用的是 `cargo xtask version check <tag>` 与 `cargo xtask changelog notes <tag>`(`release.yml:59,63`);改这两个子命令的接口要同步 workflow(`../../xtask/automation/conventions.md` §7)。

### 3.4 发版后

- [ ] GitHub Release 页面说明与 CHANGELOG 段落一致;有二进制时附件与 `SHA256SUMS` 齐全
- [ ] docs.rs 构建成功(有库 crate 时;失败通常是 `include` 漏文件或 feature 门控的文档)
- [ ] 下一轮开发的第一条使用者可见改动,重新在 `[Unreleased]` 下开小节

---

## 4. 分支与 PR

- 主干 `main` 受保护,发版只从 `main`(`release.rs:19` `RELEASE_BRANCH`)。
- 功能分支 `feat/<短描述>`、修复 `fix/<短描述>`;合并前 rebase 到最新 `main`,fixup 提交 squash 掉,保持每个提交都能独立通过 `cargo xtask ci`。
- PR 标题即最终提交信息(Conventional Commits);描述从**使用者视角**写变化,方便直接搬进 CHANGELOG。
- Review 清单见 `../rust/index.md`「Quality Check · Review 时额外看」。

---

## 5. 自检清单

- [ ] 提交信息 `<type>(<scope>): 中文描述`,一个提交一件事,破坏性带 `!`
- [ ] 使用者可见改动的 CHANGELOG 条目、`Cargo.lock` 变更与代码在同一提交
- [ ] 没有手改任何成员 `Cargo.toml` / `Cargo.lock` 的版本号
- [ ] 发版前 `--dry-run` 看过计划;发版 SOP §3.1 逐项确认
- [ ] 发版失败时先判断是否已过 `publish-crates`,再决定删 tag 重推还是 bump 新版本
