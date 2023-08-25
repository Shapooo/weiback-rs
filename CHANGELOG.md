# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [v0.1.3] - 2023-08-25

**Added**
- Add mobile web login
- Support backup client only posts

**Changed**
- Save all cookies (include expired) to login_info.json
- Update version of dependencies

**Fixed**
- Unfavorite unfavorited posts break app down

## [v0.1.2] - 2023-08-05

**Added**
- Retry when http request return server error

**Changed**
- change default log level to INFO

**Fixed**
- wrong favorited status in database

## [v0.1.1] - 2023-07-22

**Added**
- github rust-clippy check action
- more information when error occur
- support status peek when unfavorite posts
- progress bar show percentage of task progress
- support for returning remaining amount of posts after a task
- github release action

**Changed**
- change some detail of UI
- use DragValue to get range of download and export
- it won't stop when get zero post from weibo

**Fixed**
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
