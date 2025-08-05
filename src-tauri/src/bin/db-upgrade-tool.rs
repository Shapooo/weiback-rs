use std::collections::HashMap;
use std::env::current_exe;
use std::path::PathBuf;

use chrono::{DateTime, FixedOffset, TimeZone};
use env_logger::Builder;
use log::{LevelFilter, error, info, warn};
use serde_json::{Value, from_str};
use sqlx::{Sqlite, SqlitePool};
use tokio::{
    fs::{File, OpenOptions, remove_file},
    io::{AsyncReadExt, AsyncWriteExt},
};
use weiback::error::{Error, Result};

#[tokio::main]
async fn main() {
    let res = start().await;
    if let Err(e) = res {
        error!("{e}");
    }
}

async fn start() -> Result<()> {
    init_logger()?;
    let db = init_db().await?;
    let user_version = check_user_version(&db).await?;
    if user_version == 2 {
        info!("Info: version fulfilled, exit.");
        return Ok(());
    } else if user_version > 2 {
        warn!("Warn: the DB file has higher version, please download newest weiback-rs!");
        return Ok(());
    } else if user_version < 0 {
        error!("Error: are you kidding? Invalid DB version");
        return Err(Error::Other("Invalid DB version".to_string()));
    }

    let mut upgrader = Upgrader::new(db).await?;
    match user_version {
        1 => {
            upgrader.upgrade_1_2().await?;
        }
        0 => {
            upgrader.upgrade_0_2().await?;
        }
        _ => unreachable!(),
    }

    info!("Upgrade succeed!");
    upgrader.close().await?;
    Ok(())
}

fn init_logger() -> Result<()> {
    let log_path = std::env::current_exe()?;
    let log_path = log_path
        .parent()
        .ok_or(Error::Other(format!(
            "the executable: {:?} should have parent, maybe bugs in there",
            std::env::current_exe()
        )))?
        .join("upgrade-db-tool.log");
    let log_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(log_path)?;
    Builder::new()
        .filter_level(LevelFilter::Info)
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();
    Ok(())
}

async fn init_db() -> Result<SqlitePool> {
    let db_url = String::from("sqlite:")
        + current_exe()?
            .parent()
            .unwrap()
            .join("res")
            .join("weiback.db")
            .to_str()
            .unwrap();
    Ok(SqlitePool::connect(db_url.as_str()).await?)
}

async fn check_user_version(db: &SqlitePool) -> Result<i64> {
    Ok(sqlx::query_as::<Sqlite, (i64,)>("PRAGMA user_version;")
        .fetch_one(db)
        .await?
        .0)
}

struct Upgrader {
    flag: i32,
    check_point: i32,
    status_file: File,
    status_file_path: PathBuf,
    db: SqlitePool,
}

impl Upgrader {
    async fn is_finished(&mut self, message: &str) -> Result<bool> {
        if self.flag > self.check_point {
            self.check_point += 1;
            Ok(true)
        } else {
            self.status_file
                .write_all(format!("{}: {}\n", self.flag, message).as_bytes())
                .await?;
            info!("{message}...");
            self.flag += 1;
            self.check_point += 1;
            Ok(false)
        }
    }

    async fn new(db: SqlitePool) -> Result<Self> {
        let (mut status_file, status_file_path) = Upgrader::create_status_file().await?;
        let mut status_content = String::new();
        status_file.read_to_string(&mut status_content).await?;
        let flag = status_content
            .lines()
            .map(|s| {
                s.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
        info!("flag {flag}");
        Ok(Self {
            flag,
            check_point: 0,
            status_file,
            status_file_path,
            db,
        })
    }

    async fn create_status_file() -> Result<(File, PathBuf)> {
        let path = current_exe()?
            .parent()
            .unwrap()
            .join("res")
            .join("DB文件升级完成前别删!");
        Ok((
            OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .append(true)
                .open(&path)
                .await?,
            path,
        ))
    }

    async fn upgrade_1_2(&mut self) -> Result<()> {
        // DB 文件1到2版本升级内容：
        // 1.将用于 Unix 时间戳记录的 INTEGER 类型 created_at 名称改为 created_at_timestamp；
        // 仍然保留 created_at 列，用于保存字符串格式的时间。
        // 2.将 retweeted_status 列更名为 retweeted_id。原因：更合理直观该列为转发的数字id，
        // 且代码里，需要用到 retweeted_status 保存整个转发。
        // 3.新增四个用户字段：rcList, customIcons, tags, complaint。
        // 这四个字段原本在 post 里。
        info!("Upgrading db from version 1 to 2, this may take a while...");
        self.add_created_at_str().await?;
        self.rename_retweeted_status_to_id().await?;
        self.add_4_columns().await?;
        if !self.is_finished("setting user_version...").await? {
            sqlx::query("PRAGMA user_version = 2")
                .execute(&self.db)
                .await?;
        }
        info!("all task finished.");
        Ok(())
    }

    async fn upgrade_0_2(&mut self) -> Result<()> {
        // DB 文件0到2版本升级内容：
        // 1.新增 created_at_timestamp 字段，INTEGER 类型，用于记录 Unix 时间戳；
        // 同时增加一个 created_at_tz 字符串类型，表示时区。
        // 因为原非标准的时间字符串格式无法使用 sqlite 进行计算，后续的按时间筛选、排序功能不方便做。
        // 2.为 users 表增加 backedup 字段。
        // 因为新增了用户备份功能，需要一个字段记录被备份的用户，方便后续导出功能。
        // 3.新增了 picture 表。
        // 原本的 picture_blob 表无法体现图片与 posts/users 的关系，只有存储图片的基本功能。
        // 新增关系之后，方便后续删除冗余图片等功能的实现。
        // 4.将 retweeted_status 列更名为 retweeted_id。原因：更合理直观该列为转发的数字id，
        // 且代码里，需要用到 retweeted_status 保存整个转发。
        // 5.新增四个用户字段：rcList, customIcons, tags, complaint。
        // 这四个字段原本在 post 里。
        info!("Upgrading db from version 0 to 2, this may take a while...");
        self.add_created_at_timestamp().await?;
        self.rename_retweeted_status_to_id().await?;
        self.add_backedup_column().await?;
        self.create_picture_table().await?;
        self.add_4_columns().await?;
        if !self.is_finished("setting user_version...").await? {
            sqlx::query("PRAGMA user_version = 2")
                .execute(&self.db)
                .await?;
        }
        info!("all task finished.");
        Ok(())
    }

    async fn add_4_columns(&mut self) -> Result<()> {
        if self
            .is_finished("task: add 4 columns: rcList, customIcons, tags, complaint.")
            .await?
        {
            return Ok(());
        }
        sqlx::query(
            "ALTER TABLE posts ADD COLUMN rcList TEXT;\
             ALTER TABLE posts ADD COLUMN customIcons TEXT;\
             ALTER TABLE posts ADD COLUMN tags TEXT;\
             ALTER TABLE posts ADD COLUMN complaint TEXT;",
        )
        .execute(&self.db)
        .await?;
        Ok(())
    }

    async fn add_created_at_str(&mut self) -> Result<()> {
        if self.is_finished("task: add created_at str column.").await? {
            return Ok(());
        }
        let mut trans = self.db.begin().await?;
        sqlx::query("ALTER TABLE posts RENAME COLUMN created_at TO created_at_timestamp;")
            .execute(trans.as_mut())
            .await?;
        sqlx::query("ALTER TABLE posts ADD COLUMN created_at TEXT;")
            .execute(trans.as_mut())
            .await?;
        let query_res = sqlx::query_as::<Sqlite, (i64, i64, String)>(
            "SELECT id, created_at_timestamp, created_at_tz FROM posts;",
        )
        .fetch_all(trans.as_mut())
        .await?;
        for (id, timestamp, tz) in query_res {
            let dt = tz
                .parse::<FixedOffset>()
                .unwrap()
                .timestamp_opt(timestamp, 0)
                .unwrap();
            sqlx::query("UPDATE posts SET created_at = ? WHERE id = ?;")
                .bind(dt.to_string())
                .bind(id)
                .execute(trans.as_mut())
                .await?;
        }
        trans.commit().await?;
        Ok(())
    }

    async fn rename_retweeted_status_to_id(&mut self) -> Result<()> {
        if self
            .is_finished("task: rename retweeted_status to retweeted_id.")
            .await?
        {
            return Ok(());
        }
        sqlx::query("ALTER TABLE posts RENAME COLUMN retweeted_status TO retweeted_id;")
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn add_created_at_timestamp(&mut self) -> Result<()> {
        if self.is_finished("task: add created_at column.").await? {
            return Ok(());
        }

        let mut trans = self.db.begin().await?;
        let post_data =
            sqlx::query_as::<Sqlite, (i64, String)>("SELECT id, created_at FROM posts;")
                .fetch_all(trans.as_mut())
                .await?;
        sqlx::query(
            "ALTER TABLE posts ADD COLUMN created_at_timestamp INGETER;\
                 ALTER TABLE posts ADD COLUMN created_at_tz TEXT;",
        )
        .execute(trans.as_mut())
        .await?;
        for (id, datetime) in post_data
            .into_iter()
            .map(|(id, created_at)| parse_created_at(created_at.as_str()).map(|dt| (id, dt)))
            .collect::<Result<Vec<_>>>()?
        {
            sqlx::query(
                "UPDATE posts SET created_at = ?, created_at_timestamp = ?, created_at_tz = ? WHERE id = ?;",
            )
            .bind(datetime.to_string())
            .bind(datetime.timestamp())
            .bind(datetime.timezone().to_string())
            .bind(id)
            .execute(trans.as_mut())
            .await?;
        }
        trans.commit().await?;
        Ok(())
    }

    async fn add_backedup_column(&mut self) -> Result<()> {
        if self.is_finished("task: add backedup column.").await? {
            return Ok(());
        }
        const ADD_BACKEDUP_COLUMN: &str =
            "ALTER TABLE users ADD COLUMN backedup BOOLEAN DEFAULT false;";
        sqlx::query(ADD_BACKEDUP_COLUMN).execute(&self.db).await?;
        Ok(())
    }

    async fn create_picture_table(&mut self) -> Result<()> {
        const PIC_TYPE_AVATAR: u8 = 0;
        const PIC_TYPE_INPOST: u8 = 1;
        const PIC_TYPE_EMOJI: u8 = 2;
        const CREATE_PICTURE: &str = "CREATE TABLE IF NOT EXISTS picture(\
                                  id VARCHAR PRIMARY KEY, uid INTEGER, post_id INTEGER, type INTEGER);";

        if self.is_finished("task: create picture table.").await? {
            return Ok(());
        }

        let mut trans = self.db.begin().await?;
        sqlx::query(CREATE_PICTURE).execute(trans.as_mut()).await?;
        let mut user_avatars = HashMap::new();
        let res = sqlx::query_as::<Sqlite, (i64, String, String, String)>(
            "SELECT id, profile_image_url, avatar_large, avatar_hd FROM users;",
        )
        .fetch_all(trans.as_mut())
        .await?;
        res.into_iter().for_each(|(id, url1, url2, url3)| {
            user_avatars.insert(strip_url_queries(&url1).to_string(), id);
            user_avatars.insert(strip_url_queries(&url2).to_string(), id);
            user_avatars.insert(strip_url_queries(&url3).to_string(), id);
        });
        let mut post_pic = HashMap::new();
        let res = sqlx::query_as::<Sqlite, (i64, String)>("SELECT id, pic_ids FROM posts;")
            .fetch_all(trans.as_mut())
            .await?;
        res.into_iter().for_each(|(id, ids)| {
            if ids.is_empty() {
                return;
            }
            if let Value::Array(ids) = from_str(&ids).unwrap() {
                for pid in ids {
                    if let Value::String(pid) = pid {
                        post_pic.insert(pid, id);
                    }
                }
            }
        });
        let mut idx = 0;
        loop {
            let sql = format!(
                "SELECT url, id FROM picture_blob LIMIT 1000 OFFSET {};",
                idx * 1000
            );
            idx += 1;
            let pics = sqlx::query_as::<Sqlite, (String, String)>(&sql)
                .fetch_all(trans.as_mut())
                .await?;
            if pics.is_empty() {
                break;
            }
            for (url, pic_id) in pics {
                if let Some(id) = post_pic.get(&pic_id) {
                    sqlx::query(
                        "INSERT OR IGNORE INTO picture (id, post_id, type) VALUES (?, ?, ?);",
                    )
                    .bind(pic_id)
                    .bind(id)
                    .bind(PIC_TYPE_INPOST)
                    .execute(trans.as_mut())
                    .await?;
                } else if let Some(id) = user_avatars.get(&url) {
                    sqlx::query("INSERT OR IGNORE INTO picture (id, uid, type) VALUES (?, ?, ?);")
                        .bind(pic_id)
                        .bind(id)
                        .bind(PIC_TYPE_AVATAR)
                        .execute(trans.as_mut())
                        .await?;
                } else if is_emoji(url.as_str()) {
                    sqlx::query("INSERT OR IGNORE INTO picture (id, type) VALUES (?, ?);")
                        .bind(pic_id)
                        .bind(PIC_TYPE_EMOJI)
                        .execute(trans.as_mut())
                        .await?;
                } else {
                    warn!(
                        "Warn: the post/user/emoji not found, which associated to picture {pic_id}: {url}"
                    );
                }
            }
        }
        trans.commit().await?;
        Ok(())
    }

    async fn close(self) -> Result<()> {
        let _ = self.status_file;
        self.db.close().await;
        remove_file(self.status_file_path).await?;
        Ok(())
    }
}

pub fn strip_url_queries(url: &str) -> &str {
    url.split('?').next().unwrap()
}

pub fn parse_created_at(created_at: &str) -> Result<DateTime<FixedOffset>> {
    Ok(DateTime::parse_from_str(created_at, "%a %b %d %T %z %Y")?)
}

use once_cell::sync::Lazy;
use regex::Regex;
static EMOJI_URL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^https://face.t.sinajs.cn.*").unwrap());

fn is_emoji(url: &str) -> bool {
    EMOJI_URL_RE.is_match(url)
}
