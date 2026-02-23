# Changelog

All notable changes to this project will be documented in this file.

## [v0.3.0-alpha] - 2026-02-23

### Added
- **全新的用户界面**: 前端由 egui 重构为 `Tauri` + `React`，提供更现代、更丰富的用户体验。
- **视频与 LivePhoto 备份**: 添加了对视频和 LivePhoto 内容的备份支持。
- **独立的数据库迁移工具**: 为 v0.2.x 版本的数据库提供升级到新版数据库的功能。
- **单个微博重新备份/删除**: 支持对已备份的单个微博进行重新备份或彻底删除（包括相关资源）。
- **任务失败记录与提示**: 增强了任务管理，现在可以记录并提示失败的子任务。
- **丰富的微博内容展示**:
    - 支持表情（emoji）显示。
    - 微博中的内嵌图片现在可以直接显示。
    - 提供了大图查看器，支持拖拽和缩放。
- **开发模式**: 新增为开发者设计的模式，可以保存所有API原始数据以供调试。

### Changed
- **完全重构的后端**: 后端代码被彻底重构，拆分为 `core`、`storage`、`exporter`、`network` 等多个高内聚、低耦合的模块。
- **新的任务管理核心**: 引入 `Core` 和 `TaskManager` 模块，将核心业务逻辑与UI分离，提升了稳定性和可维护性。
- **统一的在线备份页面**: 合并了“用户备份”和“收藏备份”页面，简化了操作流程。
- **改进的API数据处理**: 增强了对微博API返回数据的解析能力，对不规范和非预期的字段有更好的兼容性。
- **新的数据库结构**:
    - 使用 `sqlx-cli` 管理数据库迁移。
    - 新增 `picture`, `video` 等表，以更合理地管理多媒体资源。
    - 新增 `favorited_posts` 表，用于记录所有收藏过的微博，避免重复备份。
- **配置文件格式**: 配置文件从 `json` 格式更换为 `toml`，可读性更强。
- **HTML导出重构**: 导出逻辑被重构，现在使用编译期读取的模板，并且可以更灵活地处理微博内容。

### Fixed
- 修复了若干微博API数据解析中的潜在崩溃问题。
- 修复了备份范围计算不正确的问题。
- 修复了数据库迁移工具中 `unfavorited` 状态迁移不正确的问题。

## [v0.2.5] - 2025-01-17

### Fixed

- 修复登录闪退问题

## [v0.2.4] - 2024-07-09

### Added

- README 中添加高级功能使用教程

### Fixed

- 取消收藏时，因已经取消的收藏导致的报错直接忽略
- posts.html 模板增加更仔细的字段检查，避免因字段不存在导致的闪退

### Changed

- 更新依赖版本
- 依 clippy 建议进行修改
- 设置默认日志等级到 debug，方便问题排查

## [v0.2.3] - 2024-02-27

### Fixed

- 修复数据库初始化过程中 pragma 导致的错误

### Changed

- 取消收藏失败将不再影响其它任务

## [v0.2.2] - 2024-02-26

### Fixed

- 修复因没有提取转发头像导致的导出时报错

### Changed

- task handler 初始化阶段的报错发送到主线程，以便日志记录

## [v0.2.1] - 2024-01-19

### Fixed

- 修复手机版网页的Post和User的部分字段可能不存在导致的错误

## [v0.2.0-patch1] - 2024-01-09

### Changed

- 收藏备份时最后一页不再等待

### Fixed

- 修复用户备份获取完整内容时可能失败的问题

## [v0.2.0] - 2024-01-09

### Added

- 新增了备份用户时显示名字和头像的功能

### Changed

- 非常多的重构，对80%的代码重构，模块划分更合理
- 微博数据处理的部分完全放到 Post 里
- 用户数据处理的部分完全放到 User 里
- 图片下载、存储部分完全放到 picture 模块里
- 移除 post_processor 模块
- 使用 channel 而不是共享内存进行 ui 和工作线程通信
- UI 会主动刷新
- 数据库字段变更
- ...

## [v0.1.6] - 2023-12-30

### Added

- 新增了备份自己和指定用户微博的功能
- 新增了数据库升级工具

### Changed

- 数据库变动：created_at 字段变为使用时间戳
- 数据库变动：用户增加 backedup 字段
- 数据库变动：增加 Picture 表

### Fixed

- 修复了因移动端微博API变动导致的无法登录的问题

### NOTICE

- 老用户需要升级数据库，详见 README

## [v0.1.5] - 2023-12-10

### Changed

- filter unecessary module logs
- update version of dependencies

### Fixed

- export range wrong

## [v0.1.4] - 2023-08-26

### Added

- Support to url_objects of data from mobile web

### Changed

- Use another api for client-only posts
- Change log level of fetching client-only post from debug to info

### Fixed

- clippy warnings
- forget to mark client-only post favorited
- wrong field type in post json
- post should be inserted after url_struct procession

## [v0.1.3] - 2023-08-25

### Added

- Add mobile web login
- Support backup client only posts

### Changed

- Save all cookies (include expired) to login_info.json
- Update version of dependencies

### Fixed

- Unfavorite unfavorited posts break app down

## [v0.1.2] - 2023-08-05

### Added

- Retry when http request return server error

### Changed

- change default log level to INFO

### Fixed

- wrong favorited status in database

## [v0.1.1] - 2023-07-22

### Added

- github rust-clippy check action
- more information when error occur
- support status peek when unfavorite posts
- progress bar show percentage of task progress
- support for returning remaining amount of posts after a task
- github release action

### Changed

- change some detail of UI
- use DragValue to get range of download and export
- it won't stop when get zero post from weibo

### Fixed

- follow rust-clippy, fix coding style problem
- wrong favorite status in database

## [v0.1.0] - 2023-07-20

First Edition, with features:

- support for downloading raw data of weibo posts
- support for extraction img url from weibo
- support for persist data to db
- support for exportion posts in HTML format
- support for unfavoriting posts in local
- support for login with QRCode
