# 投稿指南

开源文艺欢迎每一位创作者投稿。整个流程对非程序员友好：你只需要一个账号、装好 Git 和一个叫 `soil` 的小工具，剩下的由工具一步步引导完成，不需要学习任何 Git 命令。

## 投稿前准备

1. **一个账号**：GitHub、Gitee 或 GitCode 任选其一（投稿时工具会让你选择平台；国内网络环境建议选 Gitee 或 GitCode）
2. **安装 Git**：[git-scm.com/downloads](https://git-scm.com/downloads)。Windows 安装时记得勾选「Git from the command line and also from 3rd-party software」
3. **安装 soil 工具**：见下节

## 安装 soil

soil 是一个单文件命令行工具，安装后在任何目录下打开终端即可运行。

### Windows

打开 PowerShell，粘贴运行：

```powershell
irm https://github.com/windwhiterain/open-literature-and-art/releases/latest/download/install.ps1 | iex
```

卸载：

```powershell
Remove-Item "$env:LOCALAPPDATA\soil" -Recurse -Force
```

### macOS / Linux

打开终端，粘贴运行：

```sh
curl -fsSL https://github.com/windwhiterain/open-literature-and-art/releases/latest/download/install.sh | sh
```

卸载：

```sh
rm ~/.local/bin/soil
```

安装完成后运行 `soil`，出现帮助信息即安装成功。

## 构建你的本地文稿库

「文稿库」就是存放你所有文稿的一个文件夹。按下面的规则整理好后，`soil` 会自动识别哪些是独立文章、哪些是文集章节：

```
my-writing/
├── 随笔.md              ← 独立文章：文稿库根目录下的 .md 文件
├── 随笔.soil.toml       ← 随笔.md 的元数据（可选，见下）
├── 一个故事/             ← 目录里只有一篇同名 .md → 独立文章，目录放附件
│   ├── 一个故事.md
│   └── 插图.png
└── 长篇小说/             ← 目录里有多篇 .md → 文集，每篇是一个章节
    ├── 第一章.md
    ├── 第二章.md
    └── 第一章.soil.toml  ← 单章元数据（可选）
```

**识别规则**：

- **独立文章**：文稿库根目录下的 `.md` 文件；或一个目录中只有一篇与目录同名的 `.md`（此时目录可放插图等附件）
- **文集**：一个目录中有多篇 `.md`，每篇作为一个章节
- `README.md` 会被当作作者页的简介，不会列为文章

**元数据（可选）**：每个文章/文集旁可放一个同名 `.soil.toml` 文件，记录创作时间、简介、是否草稿。在文稿库目录下运行：

```sh
soil meta
```

它会自动为所有文章/文集生成 `.soil.toml` 模板（已存在的不会被覆盖）。按需填写：

```toml
date = "?~2026-08-26"  # 创作时间；"?~" 表示由工具按文件修改时间推测、未经确认，可改成真实日期
draft = false           # true = 草稿，不会发布到网站
summary = ""            # 一句话简介，显示在文章列表卡片上
```

## 投稿

在你的文稿库目录下打开终端，运行：

```sh
soil submit
```

工具会一步步引导你完成：

1. **选择平台**——GitHub / Gitee / GitCode 三选一，之后自动记住
2. **登录**——在浏览器中授权一次，之后自动记住（GitHub 走设备码授权，Gitee / GitCode 打开浏览器授权）
3. **整理文稿仓库**——工具会创建或复用你的文稿仓库：如果文稿文件夹还不是 Git 仓库，它会帮你建好并推送；以后每次投稿直接复用
4. **提交并审核**——新作者通过 Pull Request 提交到本站审核；已收录作者则直接更新自己的仓库，网站每天自动拉取最新内容
5. **本地预览**——提交后工具自动在本地构建网站并打开浏览器，先让你看到发布后的效果

## 命令详解

| 命令 | 作用 |
| --- | --- |
| `soil` | 显示帮助 |
| `soil submit` | 完整投稿流程：登录 → 文稿仓库 → 提交 → 本地预览 |
| `soil preview [目录]` | 纯本地构建 + 浏览器预览（不做任何 git 操作，默认当前目录） |
| `soil meta` | 为当前目录的文稿生成 `.soil.toml` 元数据模板 |
| `soil -V` / `--version` | 显示版本号 |

**几条约定**：

- **退出**：任何输入处按 `Esc`（再按回车）退出
- **复制**：`Ctrl+C` 是终端复制快捷键，不会退出程序
- **网络不稳定**：工具会自动无限重试，网络恢复后自动继续；如果你的账号配置了 SSH 密钥，工具会自动改用 SSH 通道加速
- **多平台支持**：你的文稿仓库和本站收录的链接都按你选择的平台创建，国内平台（Gitee / GitCode）的网络访问更稳定

## 投稿后多久上线

这里有两次「提交」，请注意区分：

1. **你提交到自己的文稿仓库**——运行 `soil submit` 时完成，即刻生效，可以在自己的仓库看到最新内容
2. **网站从你的仓库同步**——这是另一个独立步骤。新作者第一次需要管理员审核合并；已收录作者由网站每天自动拉取一次，更新在 24 小时内上线

总而言之：你的仓库立刻更新，网站不是立刻更新。这是设计如此，并非 bug。

## 常见问题

**GitHub 授权页面打不开？**
网络对 GitHub 的访问不稳定时可能出现。可以稍后重试、使用代理，或者改用 Gitee / GitCode 平台投稿。

**我的文稿会公开吗？**
是的——你的文稿仓库会以公开仓库形式创建（网站构建需要公开访问），提交后文章会展示在网站上。如果不想公开某些内容，请不要放入文稿文件夹。

**我不懂 Git 能投稿吗？**
能。`soil submit` 替你完成所有 Git 操作，你只需要按提示输入路径和作者名。

## 相关

- [关于本站](about.html)——站点的运作方式与设计理念
- 本站源代码：[github.com/windwhiterain/open-literature-and-art](https://github.com/windwhiterain/open-literature-and-art)
