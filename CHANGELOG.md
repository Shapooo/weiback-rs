# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

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
