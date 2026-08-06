# 开源文艺

一个面向中文文学创作者的开源社区：把写作者的 markdown 文稿构建成漂亮的静态网站，发布在 GitHub Pages 上。

本仓库包含两个部分：

| 组件        | 说明                                                          |
| ----------- | ------------------------------------------------------------- |
| **sitegen** | 静态站点生成器：把 `authors/<作者>/` 下的 markdown 构建为网站 |
| **soil**    | 投稿工具（单二进制）：帮助不熟悉技术的作者把文稿提交到本站    |

网站地址：<https://windwhiterain.github.io/open-literature-and-art/>

---

## soil 是什么？

**soil 是为非程序员设计的投稿工具**——你不需要会 Git、不需要会编程，只要：

1. 有一个 GitHub 账号
2. 电脑上装了 [Git](https://git-scm.com/downloads)

在你的文集目录下运行 `soil submit`，它会一步步引导你：

1. **登录 GitHub**（在浏览器里授权一次，之后自动记住）
2. **自动处理一切**：创建/使用你的文稿仓库 → 提交文稿 → 提交到本站审核
3. **本地预览**：自动构建网站并在浏览器中打开，让你先看到效果
4. **每日自动同步**：已收录的作者之后更新文稿只需提交到自己的仓库（`soil submit` 会帮你做），无需重复走审核流程。网站每天自动从你的仓库拉取最新内容——你的文稿不会立刻出现在网站上，而是在一天内上线

## 安装

> 最新版本在 [Releases](https://github.com/windwhiterain/open-literature-and-art/releases/tag/latest)。

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

## 使用方法

在你的文稿库下打开终端：

```sh
soil submit
```

按提示操作即可。几条约定：

- **退出**：任何输入处按 `Esc`（再按回车）退出
- **复制**：`Ctrl+C` 是终端复制快捷键，不会退出程序
- **网络不稳定**：工具会自动无限重试，网络恢复后自动继续；如果你的 GitHub 账号配置了 SSH 密钥，工具会自动改用 SSH 通道加速

### 其他命令

```sh
soil               # 显示帮助
soil preview       # 将你的文稿库在本地构建成网站并打开浏览器预览（不做任何 git 操作）
soil meta          # 为当前目录的文稿生成 .soil.toml 元数据模板，包括创作时间，作品简介
soil -V, --version # 显示版本号
```

### 使用示例

```
$ soil submit

═══ 文稿目录与仓库 ═══

你的文稿文件夹将直接作为仓库的工作副本。
请输入你的文稿文件夹路径（直接回车使用当前目录） [.] C:\Users\me\my-writing
使用文稿目录: C:\Users\me\my-writing
...
═══ 本地预览 ═══
正在构建网站...
已打开：...\dist\index.html
```

### 前置要求

- [Git](https://git-scm.com/downloads)——工具首次运行会检测；Windows 安装时记得勾选「Git from the command line and also from 3rd-party software」
- 一个 [GitHub 账号](https://github.com/signup)——没有的话按提示在浏览器里注册

## 常见问题

**GitHub 授权页面打不开？**
网络对 GitHub 的访问不稳定时可能出现。可以稍后重试，或使用代理软件后再运行。

**我的文稿会公开吗？**
是的——你的文稿仓库会以公开仓库形式创建（网站构建需要公开访问），提交后文章会展示在网站上。如果不想公开某些内容，请不要放入文稿文件夹。

**投稿后多久能上线？**

这里有两次「提交」，请注意区分：

1. **你提交到自己的文稿仓库**——运行 `soil submit` 时完成，即刻生效。你可以在自己的 GitHub 仓库看到最新内容。
2. **网站从你的仓库同步**——这是另一个独立步骤。如果你是新作者，第一次需要管理员审核合并。如果你已经是已收录的作者，网站每天自动拉取一次你的仓库内容，你的更新会在 24 小时内上线。

总而言之：你的仓库立刻更新，网站不是立刻更新。这是设计如此，并非 bug。
