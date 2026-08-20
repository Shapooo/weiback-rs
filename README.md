<p align="center">
  <img src="https://raw.githubusercontent.com/Shapooo/weiback-rs/master/weiback/resources/logo.png" alt="WeiBack Logo" width="140" />
</p>

<h1 align="center">WeiBack</h1>

<p align="center">
  <a href="https://github.com/Shapooo/weiback-rs/blob/master/LICENSE">
    <img src="https://img.shields.io/github/license/Shapooo/weiback-rs" alt="license" />
  </a>
  <a href="https://www.rust-lang.org/">
    <img src="https://img.shields.io/badge/Made%20with-Rust-1f425f.svg" alt="Made with Rust" />
  </a>
  <a href="https://tauri.app/">
    <img src="https://img.shields.io/badge/Tauri%202-24C8D8?logo=tauri&logoColor=white" alt="Tauri 2" />
  </a>
  <a href="https://github.com/Shapooo/weiback-rs">
    <img src="https://img.shields.io/github/stars/Shapooo/weiback-rs.svg?style=social&label=Star&maxAge=2592000" alt="GitHub stars" />
  </a>
</p>

WeiBack 是一款开源的微博（Weibo）数据备份桌面应用，帮助你备份、浏览和整理自己在微博上的数据。

> 注意：本项目仅为技术学习和交流，请在遵守当地相关法律法规的前提下使用本项目。

------

## 功能特性

- **在线备份**
  - 用户备份：可备份任意指定用户的微博，留空则默认备份当前登录用户；支持选择备份类型（全部 / 原创 / 图片 / 视频 / 文章）与备份页数。
  - 收藏备份：备份当前登录用户的收藏，可指定备份页数。
  - 取消已备份收藏：将本地数据库中已备份的收藏微博从微博平台上取消收藏。
- **内容浏览与批量处理**
  - 浏览本地已备份的微博，支持按用户、正文关键词（模糊 / 严格）、日期范围、收藏状态等条件筛选，并支持结果逆序排序。
  - 对筛选结果进行批量操作：导出为 HTML、重新备份、重新备份缺失图片。
  - 单条微博支持查看大图、删除与重新备份。
- **数据维护**
  - 图片清晰度去重：同一张图片存在多个清晰度时，按保留策略清理多余文件。
  - 失效头像清理：清理用户更换头像后遗留的旧头像文件。
  - 失效微博清理：清理因删除或不可抗力而失效的微博，支持深度清理模式。
  - 失效图片清理：清理下载出错或被和谐（“大眼化”）的图片。
- **本地导出**
  - 将筛选结果导出为离线 HTML 文件，可配置每个 HTML 文件包含的微博数与纯静态模式。
- **个性化设置**
  - 暗色模式、图片下载开关、图片清晰度选择、请求间隔等高级配置。

------

## 安装

### 下载预编译可执行文件

从 [Releases](https://github.com/Shapooo/weiback-rs/releases) 下载最新版本对应平台的预编译压缩包，解压即可使用。

### 从源码编译

- 克隆或下载本项目到本地
- 参考 [tauri](https://tauri.app/start/prerequisites/) 安装项目依赖
- 在项目 `tauri-app` 目录下运行 `yarn tauri build` 构建安装包，生成的安装包位于 `target/release/bundle` 目录下。

### 注意

提供 macOS 平台的可执行文件下载，但因作者不使用 macOS，不负责解决 macOS 上平台相关的 Bug。

------

## 使用

### 准备工作

> 若已经使用过旧版本，可能需要进行一些准备工作，首次使用请忽略。

- 旧版本的 `res/weiback.db` 为数据库文件，保存有备份的所有数据。
- 如需在新版本中使用该数据库文件，请下载 [Releases](https://github.com/Shapooo/weiback-rs/releases) 页面中的 `db-upgrade-tool` 进行升级。
- **使用方法**：将 `db-upgrade-tool` 可执行文件放在与旧版 `weiback.db` **相同的目录**下并执行，程序会自动寻找数据库文件并完成升级。

### 登录

首次使用需登录。进入“用户”页面，输入手机号获取短信验证码，再输入验证码即可完成登录。

### 在线备份

进入“备份”页面：

- **用户备份**：填写要备份的用户 ID（留空默认为当前登录用户），选择备份类型与页数，点击开始备份。
- **收藏备份**：填写备份页数后点击开始备份；“取消已备份收藏”会将本地已备份的收藏从微博平台上取消收藏。

### 内容浏览与导出

进入“内容浏览”页面：

- 通过用户、关键词、日期、收藏状态等条件筛选并浏览本地微博。
- 点击“导出为 HTML”选择保存目录，即可将筛选结果导出为离线网页。

### 数据维护

进入“数据维护”页面，可按需执行图片清晰度去重、失效头像清理、失效微博清理、失效图片清理等操作。

------

## 其它

WeiBack 的油猴脚本版本 [WeiBack](https://github.com/Shapooo/WeiBack) 也可在 [Greasyfork](https://greasyfork.org/zh-CN/scripts/466100-weiback) 下载安装。该版本功能较弱，仅能导出，无法保存到本地数据库，但使用方便，适合数据量不大的用户临时使用。

------

## FAQ

- 为什么备份速度这么慢？
  - 过快的接口请求频率会增加微博官方的负载，可能增加被 ban 甚至是法律风险。因此程序在请求之间增加了合理的等待时间，以模拟正常的微博访问。建议备份开始后放一边做其它事。
- 为什么下载的微博有遗漏？
  - 可能是在备份期间添加或删除了收藏，导致微博返回的数据错位。建议备份时不要在微博上进行添加或删除操作。
- 为什么微博显示收藏很多，但全部下载后发现没有那么多？
  - 部分微博因不可抗力不再可见，备份工具也无法备份这部分内容。

## 问题排查

- 程序出现问题时，首先查看 `weiback.log` 日志文件排查错误。
- 通过邮箱联系作者，或提交 Issue。

------

## 贡献

欢迎参与贡献，可以通过以下方式：

- 提交 [issue](https://github.com/Shapooo/weiback-rs/issues) 报告问题或提出建议
- 提交 [pull request](https://github.com/Shapooo/weiback-rs/pulls) 提交代码或文档

## 开源协议

本项目使用 [Apache 2.0 License](LICENSE) 开源协议。

## 联系方式

如果你有任何问题或反馈，可以通过以下方式联系作者：

- 邮箱：<shabojia@outlook.com>