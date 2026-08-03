# AGENTS.md

Rust 静态站点生成器：把 `authors/<author>/` 下的中文 markdown 文学作品构建为 GitHub Pages 静态站点。无模板引擎、无框架，HTML 用 `format!` 生成，随机逻辑在浏览器端 JS。

## 构建与验证命令

```sh
cargo run -- --out-dir dist            # 构建站点到 dist/
cargo run -- --out-dir dist --check-links  # 构建 + 校验所有 href/src 无断链（CI 同款）
cargo test                             # 9 单元测试 + 1 集成测试（tests/build_site.rs，临时目录 fixture）
cargo fmt --check                      # CI 门槛
cargo clippy -- -D warnings            # CI 门槛
```

CI 门槛顺序：`fmt --check` → `clippy -D warnings` → `test` → `run --check-links`。所有验证需通过才能声称完成。

## 核心业务规则（scan.rs）

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
- `--check-links` 扫描 `href="` / `src="` 模式；CSS 常量里的类名不影响（只匹配属性写法）
- Windows 开发环境：输出路径一律用 `/` 拼接，不要依赖 `Path::display()`

## CI / 部署

`.github/workflows/deploy.yml`：
- 触发分支是 **`v1`**（不是 main），`workflow_dispatch` 也可
- checkout 必须 `submodules: recursive`（`authors/whiting` 是 git 子模块，内容来自外部仓库）
- 上传 `dist/` → GitHub Pages；仓库 Settings → Pages → Source 需手动设为 "GitHub Actions"（YAML 无法配置）
- `dist/` 和 `target/` 已 gitignore
