# AGENTS.md

Cargo workspace，两个 crate：
- **sitegen**（根，`src/`）：静态站点生成器，把 `authors/<author>/` 下的中文 markdown 构建为 GitHub Pages 静态站点。无模板引擎，HTML 用 `format!` 生成，随机逻辑在浏览器端 JS。同时是 lib（`sitegen::run`）和 bin。
- **soil**（`soil/`）：CLI 工具，帮非程序员把文稿提交到本仓库。**内嵌 sitegen 库**（`soil/Cargo.toml` 依赖 `sitegen = { path = ".." }`），用户只需单个 soil 二进制。命令系统：无参数/`-h`/`--help` 显示帮助，`-V`/`--version` 显示版本，未知命令报错并退出码 2。子命令：
  - `soil submit`（原默认流程）：登录 → 文稿仓库 → fork/直推 → git submodule → PR → 本地预览
  - `soil preview [目录]`：纯本地构建 + 浏览器预览，**零 git 操作**，默认当前目录，要求目录含 `authors/`；复用 `wizard.rs::sitegen_build_and_open`
  - `soil meta`：生成 `.soil.toml` 模板，不联网。

## 构建与验证命令

```sh
cargo run -- --out-dir dist            # 构建站点到 dist/（根 member 默认 sitegen）
cargo run -- --out-dir dist --check-links  # 构建 + 校验所有 href/src 无断链（CI 同款）
cargo test                             # sitegen 8 单元测试 + 1 集成测试（tests/build_site.rs，临时目录 fixture）
cargo build -p soil                    # soil 无测试，编译通过即可
cargo fmt --check                      # CI 门槛
cargo clippy -- -D warnings            # CI 门槛（两个 crate 一起查）
```

CI 门槛顺序：`fmt --check` → `clippy -D warnings` → `test` → `run --check-links` → `build -p soil`。所有验证需通过才能声称完成。

## 上游仓库（soil 相关，容易猜错）

- 上游 = **`windwhiterain/open-literature-and-art`**（main.rs `UPSTREAM_OWNER/REPO` 硬编码；不存在 open-literature-and-art 组织）
- **v1 分支不是 default branch**——所有操作必须在 v1 上：clone 用 `-b v1`，已有副本先 `checkout v1`，PR 的 head/base 都是 v1
- 仓库 owner（windwhiterain）是维护者：soil 用 `permissions.push` 检测（`has_push_permission`），维护者**跳过 fork/PR 直接 push 上游 v1**，普通用户 fork + PR

## soil 核心业务规则（wizard.rs）

**文稿仓库决策**（先问目录路径，按目录状态分流）：
- 目录是 git 仓库且有 origin → 复用（`repo_from_url` 解析，支持 https/ssh/owner-repo 格式）
- 目录是 git 仓库无 origin → 问仓库 URL
- 目录不是 git 仓库 → 创建新仓库；`create_repo` 报 422 `already exists` → **覆盖**：普通 push 失败后 `pull_remote_ours`（`git pull --allow-unrelated-histories -s ours`，保留远端历史、内容=本地目录），**绝不 force push**

**作者名决策**：读本地 work_dir `.gitmodules` 判断——repo URL 已收录 → 自动更新（`submodule update --init --remote`）；未收录 → 问作者名。**名字冲突在提交时检测**（`is_name_conflict` 匹配 git 的 `already exists`），提示换名字重试，不在本地预检（fork 可能落后上游）。

**提交去重**（防重复提交/空提交）：
- `is_ahead_of_upstream`（`git rev-list --count @{u}..HEAD`）——无未推送提交则跳过 push（空 push 在网络差时会挂住）
- `has_staged_changes`——无变更则跳过 commit/push/PR（更新模式重跑时子模块可能已最新）

**登录**：OAuth device flow（`github.rs` CLIENT_ID 硬编码，勿改）；token 持久化到 `{data_dir}/soil/token.txt`（Windows: `%APPDATA%\soil\`），启动时 `load_token` + `/user` 验证复用。缓存目录 `{data_dir}/soil/`：`fork/`（本地工作副本）、`token.txt`。

**本地预览**（提交流程末尾）：`submodule_init` 只拉**当前作者**子模块（`git submodule update --init`，用 gitlink 记录的 commit，不加 `--remote`）→ 进程内 `sitegen::run` 构建 → `cmd /c start` 打开 `dist/index.html`。

## soil 交互与网络约定

- **Ctrl+C 被 ctrlc handler 吞掉**（终端里它是复制快捷键），进程不退出；退出键是 **Esc**（`is_quit` 检测 `\u{1b}` 前缀，行缓冲下按 Esc 后回车）。所有 y/n 确认已移除（直接继续）
- **网络类 git 操作无限重试**（`run_git_network`：clone/fetch/pull/push/submodule 等），每轮打印"第 N 次失败"；`is_network_error` 分类（unable to access/connection/timeout/reset/ssl 等）——认证失败、仓库不存在等**非网络错误立即报出**。本地操作（add/commit/checkout/config/rev-list）不重试
- **SSH 自动探测**（`ssh_available`：`~/.ssh` 有 key + `ssh -T -o BatchMode=yes -o ConnectTimeout=10` 认证成功）→ 配置 `url.git@github.com:.insteadOf https://github.com/`（仅本地 config，不提交；`.gitmodules` 里的 URL 保持 HTTPS，CI 无凭据克隆需要）
- 无限重试期间无法用 Ctrl+C/Esc 停止（没有输入点），只能关终端

## 核心业务规则（sitegen scan.rs）

内容分类——**这是最容易改错的地方**：

- 作者目录下顶层 `.md` → 独立文章
- 子目录**仅 1 个 `.md` 且文件名（去 .md）== 目录名** → 独立文章（目录是它的附件容器），例如 `杨朔的愿望/杨朔的愿望.md`
- 子目录有 ≥2 个 `.md`，或唯一 `.md` 与目录名不同 → 文集（章节列表），例如 `地方教育/大纲.md`
- `README.md` → 作者页介绍，不列为文章
- 非 `.md` 文件 → 资源，复制到 `files/<author>/...`

## 链接约定（render.rs / paths.rs）

- **所有链接必须显式指向具体文件**（`./xxx/index.html`），禁止目录链接（`./xxx/`）——`file://` 下会显示目录列表而不是页面（已修过的 bug）
- **只用相对路径**，禁止 `/index.html` 式绝对路径——子路径部署（`user.github.io/repo/`）下会错；depth 0 时直接写 `index.html`
- URL 用原始 UTF-8 文件名，不做 slug 化
- 首页/作者页的随机 10 项 + 「换一批」是**客户端 JS**（页面内嵌 `items` 数组，`js_string()` 转义），构建时不做随机，无 `--seed`

## 排序

列表（作者页混合列表、文集章节列表）按名字**字典序**（`String::cmp`，Unicode 码点），不是自然序。章节页「上一章/下一章」也基于此顺序——注意 "第十章" 会排在 "第二章" 前。

## 页面结构约定

- 所有页面顶部导航栏：站点名「开源文艺」+ 文章/作者按钮，当前页加 `class="active"`
- 内容页（文章/文集/章节）标题下方必须有 `作者: [链接]`（`.author-line`）
- 底部：**仅章节页**有「← 上一章 / 下一章 →」（`.chapter-nav`，首末章用空 `.placeholder` 占位）；独立文章页和文集页底部无任何链接
- 面包屑保留（首页 / 作者 / 文集 / 章节）

## 陷阱

- **UTF-8**：遍历 HTML 字节时必须按字符推进（`bytes[i] as char` 会破坏中文——已修过的 bug）。`content.rs::rewrite_links` 做本地图片路径重写，测试必须覆盖中文内容
- **BOM**：`render_markdown` 开头 `trim_start_matches('\u{feff}')`——Windows 记事本/PowerShell 保存 UTF-8 会写 BOM，不剥会导致首行 `#` 标题失效（已修过的 bug，有回归测试）
- `--check-links` 扫描 `href="` / `src="` 模式；CSS 常量里的类名不影响（只匹配属性写法）
- **子模块残留**：上游移除某子模块（删 `.gitmodules` 条目 + gitlink）后，已 checkout 的 `authors/<name>/` 目录（含 `.git` 文件）会留在工作副本里，下次 `git add -A` 会把它重新暂存成无 URL 的 gitlink，CI 的 `submodule update` 直接报 "No url found for submodule path"（已修过的 bug：`soil/src/git.rs::remove_stale_submodules` 在暂存前清理；改 soil 的提交逻辑时不要绕过它）
- Windows 开发环境：输出路径一律用 `/` 拼接，不要依赖 `Path::display()`
- soil 无测试；改 soil 后验证 = build + fmt + clippy

## CI / 部署

`.github/workflows/deploy.yml`：
- 触发分支是 **`v1`**（不是 main），`workflow_dispatch` 也可
- **dirty-only 触发**：paths 过滤——仅 `authors/**`、`.gitmodules`、`src/**`、`Cargo.toml`、`Cargo.lock`、workflow 自身变更时构建部署；只改 soil/README 等不会触发（soil 编译由 release.yml 的 `**/*.rs` 覆盖）
- checkout 必须 `submodules: recursive`（`authors/whiting`、`authors/test_user` 等是 git 子模块）
- 上传 `dist/` → GitHub Pages；仓库 Settings → Pages → Source 需手动设为 "GitHub Actions"（YAML 无法配置）
- `dist/` 和 `target/` 已 gitignore
