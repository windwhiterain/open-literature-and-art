# 开源文艺

> 打倒赛博数据地主。（指封闭 UGC 内容的互联网公司）

开源共建的小说、散文与创作档案馆。


[网页：https://windwhiterain.github.io/open-literature-and-art/](https://windwhiterain.github.io/open-literature-and-art/)

## 协议

- **代码**（模板、样式、脚本、构建配置）：[MIT](LICENSE)
- **内容**（文章、作者简介）：[CC BY-NC-SA 4.0](LICENSE)

**版权始终归作者所有**，提交不等于转让。CC BY-NC-SA 4.0 只做三件事：署名（保留你的名字）、非商业（禁止未授权的商业使用）、相同方式共享（衍生作品沿用相同协议）。商业用途请联系作者单独授权。

---

## 如何参与

向这个项目提 pull request，维护者合并以后将会自动更新网页。

### 新建一位作者

在 `content/` 下创建一个以作者名命名的文件夹，里面放 `_index.md`：

```
content/新作者/
└── _index.md
```

`_index.md` 内容：

```toml
+++
title = "新作者"
template = "author-page.html"
+++

写一段作者简介放在这里。
```

### 新建一篇文章

在作者文件夹下直接放一个 `.md` 文件，开头用 `+++` 括起来的区域放元数据：

```
content/白定/文章名.md
```

```markdown
+++
title = "文章标题"
[extra]
date = "2026-07"            # 可选
summary = "几句话摘要"        # 可选，卡片和文章页开头显示
+++

在这里写正文。
```

如果文章有配图，建一个同名目录，图片放进去，文章写成 `index.md`：

```
content/白定/文章名/
├── index.md
├── 插图.jpg
└── 封面.png
```

### 新建一部连载

在作者文件夹下创建一个目录，放 `_index.md` 作为作品首页，每章一个 `.md` 文件：

```
content/白定/我的长篇/
├── _index.md
├── 第一章.md
├── 第二章.md
```

`_index.md` 内容：

```toml
+++
title = "作品名"
sort_by = "weight"
template = "work-section.html"
[extra]
date = "2024-06"        # 可选
summary = "作品摘要"      # 可选
+++
```

每章 `.md` 内容：

```markdown
+++
title = "第一章"
weight = 1
[extra]
date = "2024-06"
+++

第一章正文。
```

### 草稿

不想发布的文章，在 frontmatter 中添加 `draft = true`：

```toml
+++
title = "未完成"
draft = true
+++
```

---

## 本地预览

安装 [Zola](https://www.getzola.org/)：

```bash
zola serve
```

## 安装小助手 soil

**Linux / macOS：**

```bash
curl -fsSL https://raw.githubusercontent.com/windwhiterain/open-literature-and-art/master/install.sh | bash
```

**Windows (PowerShell)：**

```powershell
irm https://raw.githubusercontent.com/windwhiterain/open-literature-and-art/master/install.ps1 | iex
```

安装完成后运行 `soil --help` 查看用法。
