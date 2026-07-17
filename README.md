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

然后在这个文件夹里新建文章即可。

### 新建一篇文章

在作者的文件夹下创建一个与文章同名的目录：

```
content/白定/文章名/
├── meta.toml      # 元数据（title, author, date, summary, draft）
├── body.md        # 纯正文 markdown
└── image.jpg      # 图片等资源（可选）
```

`meta.toml` 内容：

```toml
title = "文章标题"
[extra]
author = "作者名"
date = "2024-06"          # 可选，纯文本："2024上半年"、"2024-06"、"2024-06-01" 都行
summary = "几句话摘要"      # 可选，卡片和文章页开头显示，可以用 AI 生成
```

正文写入 `body.md`。图片用 `![描述](image.jpg)` 引用。

### 新建一部连载

在作者的文件夹下创建一个目录，每章一个子目录：

```
content/白定/我的长篇/
├── _index.md              # 作品首页
├── 第一章/
│   ├── meta.toml          # 章节元数据
│   └── body.md            # 章节正文
├── 第二章/
│   ├── meta.toml
│   └── body.md
```

`_index.md` 内容：

```toml
+++
title = "作品名"
sort_by = "weight"
template = "work-section.html"
[extra]
author = "作者名"
date = "2024-06"        # 可选
summary = "作品摘要"      # 可选，卡片和作品首页显示
+++
```

每章 `meta.toml`（author 和 summary 在 `_index.md` 上定义）：

```toml
title = "第一章"
weight = 1
```

正文写入 `body.md`。

### 草稿

不想发布的文章，在 `meta.toml` 中添加 `draft = true`：

```toml
title = "未完成"
draft = true
[extra]
author = "作者名"
```

---

## 本地预览

需要安装 [Zola](https://www.getzola.org/) 和 Python 3：

```bash
python scripts/serve.py
```

构建发布：

```bash
python scripts/build.py
```
