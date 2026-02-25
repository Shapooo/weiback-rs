# WeiBack-rs 📥

[![license](https://img.shields.io/github/license/Shapooo/weiback-rs)](https://github.com/Shapooo/weiback-rs/blob/master/LICENSE)
[![Rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)
[![GitHub stars](https://img.shields.io/github/stars/Shapooo/weiback-rs.svg?style=social&label=Star&maxAge=2592000)](https://github.com/Shapooo/weiback-rs)

WeiBack-rs 是一个使用Rust🦀开发的开源软件，它可以帮助你备份自己在微博上的数据。

注意：*本项目仅为技术学习和交流，请在遵守当地相关法律法规的前提下使用本项目*

------

## 安装 💻

### 下载预编译可执行文件

从[Releases](https://github.com/Shapooo/weiback-rs/releases)下载最新版本对应平台的预编译文件压缩包，解压即可使用。

### 从源码编译

- 克隆或下载本项目到本地
- 参考 [tauri](https://tauri.app/start/prerequisites/) 安装项目依赖
- 在项目 `tauri-app` 目录下运行 `yarn tauri build` 命令构建安装包，生成的安装包文件位于 `target/release/bundle` 目录下。

### 注意

提供 MacOS 平台的可执行文件下载，但因本人不使用 MacOS，所以不负责解决 MacOS 上**平台相关**的Bug。

------

## 使用 📜

### 准备工作

注意：**若已经使用过旧版本，可能需要进行一些准备工作，首次使用请忽略**

- 旧版本的 `res/weiback.db` 为数据库文件，保存有备份的所有数据。
- 如果需要在新版本中使用该数据库文件，请下载[Releases](https://github.com/Shapooo/weiback-rs/releases)页面的 `db-upgrade-tool` 工具进行升级。
- **使用方法**：将 `db-upgrade-tool` 可执行文件放在与旧版 `weiback.db` **相同的目录**下，然后执行它。程序会自动寻找 `weiback.db` 文件并进行升级。

### 登录

- 首次使用需要登录，启动后点击登录，输入手机号和短信验证码进行登录。

### 在线备份

程序主界面分为两大功能：在线备份和本地导出。在线备份页面包含“用户备份”和“收藏备份”。

- **用户备份**:
  - 用于备份指定用户的微博。可以输入用户ID，如果为空，则默认备份当前登录用户的微博。
  - 支持选择不同的备份类型（全部、原创、图片、视频、文章）。
  - 可以指定备份的页数。
- **收藏备份**:
  - 用于备份当前登录用户的收藏。
  - 可以指定备份的页数。
  - 提供“取消已备份收藏”功能，该任务会将在本地数据库中存在的收藏微博从微博平台上取消收藏。

### 本地导出与浏览

此页面用于浏览本地已备份的数据，并支持将其导出为HTML文件。

- **浏览与筛选**:
  - 可以在此页面浏览所有已备份的微博。
  - 提供强大的筛选功能，可以根据用户ID、日期范围、是否收藏等条件进行筛选和排序。
- **导出**:
  - 点击“导出筛选结果”按钮，选择一个本地文件夹，程序会将当前筛选出的所有微博导出为HTML文件。

------

## 其它 🐵

WeiBack 的油猴脚本版本 [WeiBack](https://github.com/Shapooo/WeiBack)，也可在 [Greasyfork](https://greasyfork.org/zh-CN/scripts/466100-weiback) 下载安装。功能相比本软件较弱，仅能导出，无法保存到本地数据库。但会比较方便，适合数据不多的用户临时使用。

------

## FAQ ❓

- 为什么备份速度这么慢？
  - 因为过快的接口请求频率会增加微博官方的负载，可能增加被 ban 甚至是法律风险。因此在请求之间增加了合理的等待时间，以模拟正常的微博访问。建议备份开始后放一边做其它事。
- 为什么下载的微博有遗漏？
  - 可能是因为你在备份期间添加加或删除了收藏，导致微博返回的数据错位了。建议备份时不要在微博上进行添加或删除。
- 为什么微博显示收藏很多，但全部下载后发现没有那么多？
  - 部分微博因不可抗力不再可见，备份工具也无法备份这一部分内容。

## 问题排查 🐞

- 程序出现问题时，首先查看 `weiback.log` 日志文件排查错误。
- 通过邮箱联系我，或提交 Issue。

------

## 贡献 🤝

欢迎其他开发者参与贡献，可以通过以下方式：

- 提交 [issue](https://github.com/Shapooo/weiback-rs/issues) 报告问题或建议
- 提交 [pull request](https://github.com/Shapooo/weiback-rs/pulls) 提交代码或文档

## 开源协议 📝

本项目使用 [Apache 2.0 License](LICENSE) 开源协议。

## 联系方式 📧

如果你有任何问题或反馈，可以通过以下方式联系我：

- 邮箱：<shabojia@outlook.com>
