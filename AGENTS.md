# AGENTS.md

Cargo workspace，两个 crate：
- **sitegen**（根，`src/`）：静态站点生成器，把 `authors/<author>/` 下的中文 markdown 构建为 GitHub Pages 静态站点。无模板引擎，HTML 用 `format!` 生成，随机逻辑在浏览器端 JS。同时是 lib（`sitegen::run`）和 bin。
- **soil**（`soil/`）：CLI 工具，帮非程序员把文稿提交到本仓库。**内嵌 sitegen 库**（`soil/Cargo.toml` 依赖 `sitegen = { path = ".." }`），用户只需单个 soil 二进制。命令系统：无参数/`-h`/`--help` 显示帮助，`-V`/`--version` 显示版本，未知命令报错并退出码 2。子命令：
  - `soil submit`（原默认流程）：登录 → 文稿仓库 → fork/直推 → git submodule → PR → 本地预览
  - `soil preview [目录]`：纯本地构建 + 浏览器预览，**零 git 操作**，默认当前目录，要求目录含 `authors/`；复用 `wizard.rs::sitegen_build_and_open`
  - `soil meta`：生成 `.soil.toml` 模板，不联网。

## 构建与验证命令

```sh
cargo run -- --out-dir dist            # 构建站点到 dist/（根 member 默认 sitegen）；成功后自动开浏览器预览 dist/index.html（`CI` 环境变量存在时跳过，CI 不会弹浏览器）
cargo run -- --out-dir dist --check-links  # 构建 + 校验所有 href/src 无断链（CI 同款）
cargo test                             # sitegen 11 单元测试 + 1 集成测试（tests/build_site.rs，临时目录 fixture）
cargo build -p soil                    # soil 无测试，编译通过即可
cargo fmt --check                      # CI 门槛
cargo clippy -- -D warnings            # CI 门槛（两个 crate 一起查）
```

**裸 `cargo build` 只构建 sitegen**：根 Cargo.toml 同时是 package 和 workspace（`members = [".", "soil"]`），不带 `-p`/`--workspace` 的构建默认只构建根包，soil 会被静默跳过。构建 soil 必须 `cargo build -p soil`（release.yml 曾因 `cargo build --release` 产物缺失而失败）。

CI 门槛顺序：`fmt --check` → `clippy -D warnings` → `test` → `run --check-links` → `build -p soil`。所有验证需通过才能声称完成。

## 上游仓库（soil 相关，容易猜错）

- 上游按平台取：**`platform.rs::Platform::upstream()`** 返回 `(owner, repo)`——GitHub: `windwhiterain/open-literature-and-art`；Gitee: `xiaosijicloud/open-literature-and-art`；GitCode: `whiting/open-literature-and-art`（不存在 open-literature-and-art 组织，GitHub owner 名在国内平台未注册，国内上游是**镜像账号名**，勿改回 windwhiterain）
- **多平台镜像**：soil 支持 GitHub / Gitee / GitCode 三平台投稿（`platform.rs::Platform`）。国内镜像仓库为 **Gitee: `xiaosijicloud/open-literature-and-art`、GitCode: `whiting/open-literature-and-art`**（同名仓库，含 v1 分支）。子模块仓库 `literature-and-art` 也各有一份镜像（`xiaosijicloud` / `whiting` 名下，仅 master 分支）
- **v1 分支不是 default branch**——所有操作必须在 v1 上：clone 用 `-b v1`，已有副本先 `checkout v1`，PR 的 head/base 都是 v1
- 仓库 owner（windwhiterain，GitHub；国内平台分别对应 xiaosijicloud / whiting）是维护者：soil 用 `has_push_permission` 检测，维护者**跳过 fork/PR 直接 push 上游 v1**，普通用户 fork + PR。三平台判定方式不同（见下）

## soil 核心业务规则（wizard.rs）

**平台选择**：`step_choose_platform` 在 wizard 开头询问（1.GitHub 2.Gitee 3.GitCode），选择存 `{data_dir}/soil/config.toml`（`platform = "gitee"`，手写解析，不依赖 toml crate），下次直接复用上次平台。所有 URL（克隆/remote/子模块/PR）由 `platform.web_base()` / `api_base()` 生成，**禁止内联 `https://github.com/...`**

**文稿仓库决策**（先问目录路径，按目录状态分流）：
- 目录是 git 仓库且有 origin → 复用（`repo_from_url` 解析，支持 https/ssh/owner-repo 格式，**自动识别平台**：github.com/gitee.com/gitcode.com 与 `git@host:` 前缀）
- 目录是 git 仓库无 origin → 问仓库 URL
- 目录不是 git 仓库 → 创建新仓库；`create_repo` 报 422 `already exists` → **覆盖**：普通 push 失败后 `pull_remote_ours`（`git pull --allow-unrelated-histories -s ours`，保留远端历史、内容=本地目录），**绝不 force push**

**作者名决策**：读本地 work_dir `.gitmodules` 判断——repo URL 已收录 → 自动更新（`submodule update --init --remote`）；未收录 → 问作者名。**名字冲突在提交时检测**（`is_name_conflict` 匹配 git 的 `already exists`），提示换名字重试，不在本地预检（fork 可能落后上游）。**子模块 URL 按用户所在平台写**（`.gitmodules` 会混有 gitee/gitcode 地址；sitegen 的 `normalize_repo_url` 对 https URL 通吃，CI 从海外拉国内公开仓库没问题）

**提交去重**（防重复提交/空提交）：
- `is_ahead_of_upstream`（`git rev-list --count @{u}..HEAD`）——无未推送提交则跳过 push（空 push 在网络差时会挂住）
- `has_staged_changes`——无变更则跳过 commit/push/PR（更新模式重跑时子模块可能已最新）

**登录（多平台，oauth.rs + github.rs）**：
- GitHub：OAuth **device flow**（`platform.rs` CLIENT_ID 硬编码，勿改）——请求设备码 → 用户浏览器输码授权 → 轮询拿 token
- Gitee/GitCode：**授权码模式**（两家都不支持 device flow）——`oauth.rs::login_with_browser` 起本地回调服务器 `127.0.0.1:8765`，打印授权 URL 并自动开浏览器，回调捕获 code 换 token。**回调端口必须与注册 OAuth 应用时的 redirect_uri 一致**（`http://127.0.0.1:8765/callback`）；Gitee 的 token 参数全在 form body，GitCode 的 client_secret 在 form body、其余在 query（`oauth.rs::exchange_token` 分支处理）
- token 持久化**按平台分文件**：`{data_dir}/soil/token_github.txt` / `token_gitee.txt` / `token_gitcode.txt`（Windows: `%APPDATA%\soil\`），启动时 `load_token` + `/user` 验证复用。缓存目录 `{data_dir}/soil/`：`fork/`（本地工作副本）、`token_{平台}.txt`、`config.toml`
- **维护者判定（has_push_permission）三平台不同**：GitHub 用 repo 详情 `permissions.push`；Gitee 用 `/repos/{o}/{r}/collaborators/{username}/permission` 的顶层 `permission` 字符串（"push"/"admin"）——Gitee repo 详情**没有** permissions 字段；GitCode 用 repo 详情 `owner.login` 或 `members` 数组包含该用户名——GitCode 的 `members/{username}` 端点只支持 PUT（更新），不支持 GET

**本地预览**（提交流程末尾）：`submodule_init` 只拉**当前作者**子模块（`git submodule update --init`，用 gitlink 记录的 commit，不加 `--remote`）→ 进程内 `sitegen::run` 构建 → `cmd /c start` 打开 `dist/index.html`。

## soil 交互与网络约定

- **Ctrl+C 被 ctrlc handler 吞掉**（终端里它是复制快捷键），进程不退出；退出键是 **Esc**（`is_quit` 检测 `\u{1b}` 前缀，行缓冲下按 Esc 后回车）。所有 y/n 确认已移除（直接继续）
- **网络类 git 操作无限重试**（`run_git_network`：clone/fetch/pull/push/submodule 等），每轮打印"第 N 次失败"；`is_network_error` 分类（unable to access/connection/timeout/reset/ssl 等）——认证失败、仓库不存在等**非网络错误立即报出**。本地操作（add/commit/checkout/config/rev-list）不重试
- **SSH 自动探测**（`git.rs::ssh_available(platform)`，按平台缓存：`~/.ssh` 有 key + `ssh -T -o BatchMode=yes -o ConnectTimeout=10` 认证成功）→ 配置 `url.{ssh_host}:.insteadOf {web_base}/`（仅本地 config，不提交；`.gitmodules` 里的 URL 保持 HTTPS，CI 无凭据克隆需要）。GitHub 额外探测 443 备用通道（`ssh.github.com:443`，官方为受限网络设计）并在 443 可用时写 `core.sshCommand`；Gitee/GitCode 无 443 通道，只探测各自 host（`git@gitee.com` / `git@gitcode.com`）
- 无限重试期间无法用 Ctrl+C/Esc 停止（没有输入点），只能关终端

## 核心业务规则（sitegen scan.rs）

内容分类——**这是最容易改错的地方**：

- 作者目录下顶层 `.md` → 独立文章
- 子目录**仅 1 个 `.md` 且文件名（去 .md）== 目录名** → 独立文章（目录是它的附件容器），例如 `杨朔的愿望/杨朔的愿望.md`
- 子目录有 ≥2 个 `.md`，或唯一 `.md` 与目录名不同 → 文集（章节列表），例如 `地方教育/大纲.md`
- `README.md` → 作者页介绍，不列为文章
- 非 `.md` 文件 → 资源，复制到 `files/<author>/...`
- 作者页的 **Git 仓库链接**来自仓库根 `.gitmodules`（`load_gitmodules`/`normalize_repo_url`）：SSH 格式（`git@github.com:...`）自动转 https、去尾部 `.git`；无条目或不合法 URL → 作者页不显示该行

## 链接约定（render.rs / paths.rs）

- **所有链接必须显式指向具体文件**（`./xxx/index.html`），禁止目录链接（`./xxx/`）——`file://` 下会显示目录列表而不是页面（已修过的 bug）
- **只用相对路径**，禁止 `/index.html` 式绝对路径——子路径部署（`user.github.io/repo/`）下会错；depth 0 时直接写 `index.html`
- URL 用原始 UTF-8 文件名，不做 slug 化
- 首页/作者页的随机 10 项 + 「换一批」是**客户端 JS**（页面内嵌 `items` 数组，`js_string()` 转义），构建时不做随机，无 `--seed`

**输出目录结构**（验证线上链接时按此拼 URL）：作者页 `a/<name>/index.html`；作者目录顶层 `.md` → `a/<name>/<标题>.html`；子目录文章/文集 → `a/<name>/<标题>/index.html`；文集章节 → `a/<name>/<文集>/<章节>.html`；资源 → `files/<author>/...`（scan.rs 的 `out_path` 决定）

## 排序

列表（作者页混合列表、文集章节列表）按名字**字典序**（`String::cmp`，Unicode 码点），不是自然序。章节页「上一章/下一章」也基于此顺序——注意 "第十章" 会排在 "第二章" 前。

## 页面结构约定

- 所有页面顶部导航栏：站点名「开源文艺」+ 文章/作者/关于三个按钮，当前页加 `class="active"`
- 内容页（文章/文集/章节）标题下方必须有 `作者: [链接]`（`.author-line`）
- **列表项是卡片**：整卡可点击——作者页/文集章节列表用 `<a class="card">` 包住标题（`.card-title`）+ 日期 + 摘要；首页「文章」卡片是 `<div class="card">` + 内嵌 `.stretched` 链接（`::after` 覆盖整卡）+ 作者名链接（`.list-author`，直接显示名字无前缀）——**不要在卡片 `<a>` 里再嵌套 `<a>`（HTML 非法），首页卡片必须用 div+stretched 结构**
- **全站禁用斜体**：CSS 有全局 `em{font-style:normal}`；列表摘要/空状态/引文样式（`.list-summary`/`.empty-state`/`.summary`）均不加 `font-style:italic`
- 作者页标题下若有 `.gitmodules` 条目，渲染 `<p class="repo-line">Git 仓库: <a target="_blank" rel="noopener">`
- 底部：**仅章节页**有「← 上一章 / 下一章 →」（`.chapter-nav`，首末章用空 `.placeholder` 占位）；独立文章页和文集页底部无任何链接
- 面包屑保留（首页 / 作者 / 文集 / 章节）
- 关于页（`about.html`）内容来自 `pages/about.md`（构建时渲染为 HTML），修改内容直接编辑该 markdown 文件。`ABOUT_MD` 通过 `include_str!` 嵌入二进制——和 CSS/HTML 壳一样，不读运行时文件

## 陷阱

- **UTF-8**：遍历 HTML 字节时必须按字符推进（`bytes[i] as char` 会破坏中文——已修过的 bug）。`content.rs::rewrite_links` 做本地图片路径重写，测试必须覆盖中文内容
- **BOM**：`render_markdown` 开头 `trim_start_matches('\u{feff}')`——Windows 记事本/PowerShell 保存 UTF-8 会写 BOM，不剥会导致首行 `#` 标题失效（已修过的 bug，有回归测试）
- `--check-links` 扫描 `href="` / `src="` 模式；CSS 常量里的类名不影响（只匹配属性写法）；`http(s)`/`mailto:`/`#`/`data:` 开头的外部链接跳过不校验（作者 repo 外链不会被查）
- **子模块残留**：上游移除某子模块（删 `.gitmodules` 条目 + gitlink）后，已 checkout 的 `authors/<name>/` 目录（含 `.git` 文件）会留在工作副本里，下次 `git add -A` 会把它重新暂存成无 URL 的 gitlink，CI 的 `submodule update` 直接报 "No url found for submodule path"（已修过的 bug：`soil/src/git.rs::remove_stale_submodules` 在暂存前清理；改 soil 的提交逻辑时不要绕过它）
- Windows 开发环境：输出路径一律用 `/` 拼接，不要依赖 `Path::display()`
- soil 无测试；改 soil 后验证 = build + fmt + clippy

## CI / 部署

`.github/workflows/deploy.yml`：
- 触发分支是 **`v1`**（不是 main），`workflow_dispatch` 也可
- **dirty-only 触发**：paths 过滤——仅 `authors/**`、`.gitmodules`、`src/**`、`Cargo.toml`、`Cargo.lock`、workflow 自身变更时构建部署；只改 soil/README 等不会触发（soil 编译由 release.yml 的 `**/*.rs` 覆盖）
- checkout 必须 `submodules: recursive`（当前仅 `authors/whiting` 是子模块；`authors/test_user` 只是本地未跟踪的空目录残留，CI checkout 不存在）
- 上传 `dist/` → GitHub Pages；仓库 Settings → Pages → Source 需手动设为 "GitHub Actions"（YAML 无法配置）
- `dist/` 和 `target/` 已 gitignore

**workflow 联动（GITHUB_TOKEN 机制）**：workflow 内部用 `GITHUB_TOKEN` 发起的 push **不会触发任何其他 workflow**（GitHub 防递归设计，仅 `workflow_dispatch`/`repository_dispatch` 例外；人工 push 正常触发）。因此：
- `auto-update-submodules.yml`（每日 03:57 UTC，拉 submodule 最新提交并 push）更新后必须显式 `gh workflow run deploy.yml --ref v1` 触发部署（已实现，依赖 `actions: write` 权限）——不要改成"push 后等 deploy 自动触发"或删掉该步骤
- submodule 指针变更本身匹配 deploy.yml 的 `authors/**` paths 过滤，但事件层面已被拦掉，paths 过滤救不回来

`sync-domestic.yml`（国内外镜像双向同步，每日 04:27 UTC + `workflow_dispatch`）：
- **pull-back job**：fetch Gitee/GitCode 镜像 v1 → 把镜像领先的提交 **merge** 回 GitHub v1（`merge-base --is-ancestor` 判断是否有新提交；发生冲突就 job 失败、人工处理，**绝不覆盖/force push**）→ push 回 GitHub → 有变更时显式 `gh workflow run deploy.yml --ref v1`。**checkout 必须 `fetch-depth: 0`**——祖先判断需要完整历史，浅克隆会把祖先提交误判为"不相关历史"导致 merge 失败（已踩过的坑）。**拉取必须带 token**（`https://oauth2:${TOKEN}@...`）——Gitee 对海外 runner 的匿名 fetch 返回 401 要求认证；每家重试 3 次，仍失败不阻塞另一家（`||` 跳过，下轮自愈）
- **mirror-out job**（`needs: pull-back`）：push GitHub v1 → Gitee/GitCode 主仓库（`https://oauth2:${TOKEN}@...`，token 来自 repo secrets `GITEE_TOKEN`/`GITCODE_TOKEN`——**勿删 secrets，勿把 token 明文写进 workflow**）；子模块镜像（`literature-and-art` 的 HEAD:master）尽力而为，失败不红。**checkout 同样 `fetch-depth: 0` + `submodule-depth: 0`**——浅克隆下 push 无法验证远端 tip 是本地祖先，被当非快进拒绝（"fetch first"）
- **顺序固定**：先 pull-back 再 mirror-out（pull-back 失败 → mirror-out 跳过，避免用旧 GitHub 状态覆盖国内新提交；即使执行也是非快进被拒）
- 国内镜像 owner 与 GitHub 不同名（见"上游仓库"节），改镜像 URL 时三处一起改（本 workflow、`platform.rs::upstream()`、AGENTS.md）

`release.yml`（Build Release Binaries）要点：
- 构建命令是 `cargo build --release -p soil`（裸 build 只出 sitegen，见上）
- runner 矩阵：`macos-13` 已退役（GitHub 已移除该镜像，job 会空等直到超时），Intel x64 用 `macos-15-intel`；`macos-latest` 现为 arm64
- job 有 `timeout-minutes: 30`，runner 不可用时 30 分钟内失败而非空等 24h
