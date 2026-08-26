# 开源文艺

一个面向中文文学创作者的开源社区：把写作者的 markdown 文稿构建成漂亮的静态网站，发布在 GitHub Pages 上。

本仓库包含两个部分：

| 组件        | 说明                                                          |
| ----------- | ------------------------------------------------------------- |
| **sitegen** | 静态站点生成器：把 `authors/<作者>/` 下的 markdown 构建为网站 |
| **soil**    | 投稿工具（单二进制）：帮助不熟悉技术的作者把文稿提交到本站    |

网站地址：<https://windwhiterain.github.io/open-literature-and-art/>

**国内镜像**（腾讯云 EdgeOne Pages，免费版、国内 CDN 加速）：<https://open-literature-and-art.top/>（**当前未启用**：需完成 EdgeOne 配置并设置 `EDGEONE_API_TOKEN` 后自动开启，见 `AGENTS.md`）

## 相关页面

- **[投稿指南](https://windwhiterain.github.io/open-literature-and-art/submit.html)**——如何用 `soil` 工具投稿：安装命令、命令详解、本地文稿库如何构建。对非程序员友好，无需学习 Git
- **[关于本站](https://windwhiterain.github.io/open-literature-and-art/about.html)**——站点的运作方式与设计理念

投稿工具 `soil` 是一个单文件命令行程序，最新安装命令见网站「投稿」页。

## 内容结构

每位作者的文稿以 **Git submodule** 形式收录在 `authors/<作者>/` 下——本仓库不收录文稿本身，只记录指向作者仓库的引用。站点由 `sitegen` 构建为纯静态 HTML，部署在 GitHub Pages。

## 开发

构建站点与运行测试等命令见仓库根目录的 `AGENTS.md`。
