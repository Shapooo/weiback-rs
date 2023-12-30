use weiback_rs::utils::strip_url_queries;

use std::collections::{HashMap, HashSet};
use std::env::current_exe;
use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use sqlx::{Sqlite, SqlitePool};
use tokio::{
    fs::{remove_file, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

#[tokio::main]
async fn main() -> Result<()> {
    let db = init_db().await?;
    let user_version = check_user_version(&db).await?;
    if user_version == 1 {
        println!("Info: version fulfilled, exit.");
        return Ok(());
    } else if user_version > 1 {
        eprintln!("Warn: the DB file has higher version, please download newest weiback-rs!");
        return Ok(());
    } else if user_version < 0 {
        eprintln!("Error: are you kidding? Invalid DB version");
        return Err(anyhow::anyhow!("Invalid DB version"));
    }

    let mut upgrader = Upgrader::new(db).await?;
    upgrader.upgrade_0_1().await?;

    println!("Upgrade succeed!");
    upgrader.close().await?;
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
    async fn is_unfinished(&mut self, message: &str) -> Result<bool> {
        if self.flag > self.check_point {
            self.check_point += 1;
            Ok(false)
        } else {
            self.status_file
                .write_all(format!("{}: {}\n", self.flag, message).as_bytes())
                .await?;
            println!("{}...", message);
            self.flag += 1;
            self.check_point += 1;
            Ok(true)
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
        println!("flag {}", flag);
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

    async fn upgrade_0_1(&mut self) -> Result<()> {
        // DB 文件0到1版本升级内容：
        // 1.将字符串类型的 created_at 变更为 INTEGER 类型，用 Unix 时间戳记录；
        // 同时增加一个 created_at_tz 字符串类型，表示时区。
        // 因为原非标准的时间字符串格式无法使用 sqlite 进行计算，后续的按时间筛选、排序功能不方便做。
        // 2.为 users 表增加 backedup 字段。
        // 因为新增了用户备份功能，需要一个字段记录被备份的用户，方便后续导出功能。
        // 3.新增了 picture 表。
        // 原本的 picture_blob 表无法体现图片与 posts/users 的关系，只有存储图片的基本功能。
        // 新增关系之后，方便后续删除冗余图片等功能的实现。
        println!("Upgrading db from version 0 to 1, this may take a while...");
        self.created_at_str_to_timestamp().await?;
        self.add_backedup_column().await?;
        self.create_picture_table().await?;
        if self.is_unfinished("setting user_version...").await? {
            sqlx::query("PRAGMA user_version = 1")
                .execute(&self.db)
                .await?;
        }
        self.is_unfinished("all task finished.").await?;
        Ok(())
    }

    async fn created_at_str_to_timestamp(&mut self) -> Result<()> {
        self.is_unfinished("task: convert created_at to timestamp.")
            .await?;

        let post_data =
            sqlx::query_as::<Sqlite, (i64, String)>("SELECT id, created_at FROM posts;")
                .fetch_all(&self.db)
                .await?;
        if self
            .is_unfinished("- adding column created_at_timestamp and created_at_tz.")
            .await?
        {
            sqlx::query(
                "ALTER TABLE posts ADD COLUMN created_at_timestamp INGETER;\
         ALTER TABLE posts ADD COLUMN created_at_tz VARCHAR;",
            )
            .execute(&self.db)
            .await?;
        }
        if self
            .is_unfinished("- updating created_at_timestamp and created_at_tz.")
            .await?
        {
            for fut in post_data
                .into_iter()
                .map(|(id, created_at)| parse_created_at(created_at.as_str()).map(|dt| (id, dt)))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .map(|(id, created_at)| {
                    let tz = created_at.timezone().to_string();
                    let created_at = created_at.timestamp();
                    sqlx::query(
                    "UPDATE posts SET created_at_timestamp = ?, created_at_tz = ? WHERE id = ?;",
                )
                .bind(created_at)
                .bind(tz)
                .bind(id)
                .execute(&self.db)
                })
            {
                fut.await?;
            }
        }
        if self
            .is_unfinished("- droping created_at and renaming created_at_timestamp to created_at.")
            .await?
        {
            sqlx::query(
                "ALTER TABLE posts DROP COLUMN created_at;\
         ALTER TABLE posts RENAME COLUMN created_at_timestamp TO created_at;",
            )
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    async fn add_backedup_column(&mut self) -> Result<()> {
        self.is_unfinished("task: add backedup column.").await?;
        if self.is_unfinished("- adding backedup column.").await? {
            const ADD_BACKEDUP_COLUMN: &str =
                "ALTER TABLE users ADD COLUMN backedup BOOLEAN DEFAULT false;";
            sqlx::query(ADD_BACKEDUP_COLUMN).execute(&self.db).await?;
        }

        Ok(())
    }

    async fn create_picture_table(&mut self) -> Result<()> {
        const PIC_TYPE_AVATAR: u8 = 0;
        const PIC_TYPE_INPOST: u8 = 1;
        const PIC_TYPE_EMOJI: u8 = 2;
        const CREATE_PICTURE: &str = "CREATE TABLE IF NOT EXISTS picture(\
                                  id VARCHAR PRIMARY KEY, uid INTEGER, post_id INTEGER, type INTEGER);";

        self.is_unfinished("task: create picture table.").await?;
        if self.is_unfinished("- creating picture table.").await? {
            sqlx::query(CREATE_PICTURE).execute(&self.db).await?;
        }
        let emoticon = get_emoticon().await?;
        let mut user_avatars = HashMap::new();
        let res = sqlx::query_as::<Sqlite, (i64, String, String, String)>(
            "SELECT id, profile_image_url, avatar_large, avatar_hd FROM users;",
        )
        .fetch_all(&self.db)
        .await?;
        res.into_iter().for_each(|(id, url1, url2, url3)| {
            user_avatars.insert(strip_url_queries(&url1).to_string(), id);
            user_avatars.insert(strip_url_queries(&url2).to_string(), id);
            user_avatars.insert(strip_url_queries(&url3).to_string(), id);
        });
        let mut post_pic = HashMap::new();
        let res = sqlx::query_as::<Sqlite, (i64, String)>("SELECT id, pic_ids FROM posts;")
            .fetch_all(&self.db)
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
                .fetch_all(&self.db)
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
                    .execute(&self.db)
                    .await?;
                } else if let Some(id) = user_avatars.get(&url) {
                    sqlx::query("INSERT OR IGNORE INTO picture (id, uid, type) VALUES (?, ?, ?);")
                        .bind(pic_id)
                        .bind(id)
                        .bind(PIC_TYPE_AVATAR)
                        .execute(&self.db)
                        .await?;
                } else if emoticon.get(&url).is_some() {
                    sqlx::query("INSERT OR IGNORE INTO picture (id, type) VALUES (?, ?);")
                        .bind(pic_id)
                        .bind(PIC_TYPE_EMOJI)
                        .execute(&self.db)
                        .await?;
                } else {
                    eprintln!(
                        "Warn: the post/user/emoji not found, which associated to picture {}: {}",
                        pic_id, url
                    );
                }
            }
        }
        Ok(())
    }

    async fn close(self) -> Result<()> {
        let _ = self.status_file;
        self.db.close().await;
        remove_file(self.status_file_path).await?;
        Ok(())
    }
}

pub fn parse_created_at(created_at: &str) -> Result<DateTime<FixedOffset>> {
    Ok(DateTime::parse_from_str(created_at, "%a %b %d %T %z %Y")?)
}

use serde_json::{from_str, Value};
async fn get_emoticon() -> Result<HashSet<String>> {
    // TODO: Fetch emoticon from web, rather static resource.
    let json = from_str::<Value>(include_str!("emoticon.json"))?;
    let mut res = HashSet::new();
    let Value::Object(emoticon) = json else {
        return Err(anyhow::anyhow!("Cannot recognize the emoticon json"));
    };
    for (_, groups) in emoticon {
        let Value::Object(group) = groups else {
            return Err(anyhow::anyhow!("Cannot recognize the emoticon json"));
        };
        for (_, emojis) in group {
            let Value::Array(emojis) = emojis else {
                return Err(anyhow::anyhow!("Cannot recognize the emoticon json"));
            };
            for mut emoji in emojis {
                let Value::String(url) = emoji["url"].take() else {
                    return Err(anyhow::anyhow!("Cannot recognize the emoticon json"));
                };
                res.insert(url);
            }
        }
    }
    Ok(res)
}

#[cfg(test)]
mod tool_tests {
    use super::*;

    #[tokio::test]
    async fn emoticon() {
        get_emoticon().await.unwrap();
    }
}
