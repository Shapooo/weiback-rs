use std::env::current_exe;

use anyhow::Result;
use sqlx::{Sqlite, SqlitePool};

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

    upgrade_0_1(&db).await?;

    println!("Upgrade succeed!");
    db.close().await;
    Ok(())
}

async fn check_user_version(db: &SqlitePool) -> Result<i64> {
    Ok(sqlx::query_as::<Sqlite, (i64,)>("PRAGMA user_version;")
        .fetch_one(db)
        .await?
        .0)
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

async fn upgrade_0_1(db: &SqlitePool) -> Result<()> {
    // DB 文件0到1版本升级内容：
    // 1.将字符串类型的 created_at 变更为 INTEGER 类型，用 Unix 时间戳记录；
    // 同时增加一个 created_at_tz 字符串类型，表示时区。
    // 因为原非标准的时间字符串格式无法使用 sqlite 进行计算，后续的按时间筛选、排序功能不方便做。
    // 2.为 users 表增加 backedup 字段。
    // 因为新增了用户备份功能，需要一个字段记录被备份的用户，方便后续导出功能。
    println!("Upgrading db from version 0 to 1, this may take a while...");
    created_at_str_to_timestamp(db).await?;
    add_backedup_column(db).await?;
    sqlx::query("PRAGMA user_version = 1").execute(db).await?;
    Ok(())
}

use chrono::{DateTime, FixedOffset};
pub fn parse_created_at(created_at: &str) -> Result<DateTime<FixedOffset>> {
    Ok(DateTime::parse_from_str(created_at, "%a %b %d %T %z %Y")?)
}

async fn created_at_str_to_timestamp(db: &SqlitePool) -> Result<()> {
    let post_data = sqlx::query_as::<Sqlite, (i64, String)>("SELECT id, created_at FROM posts;")
        .fetch_all(db)
        .await?;
    sqlx::query(
        "ALTER TABLE posts ADD COLUMN created_at_timestamp INGETER;\
         ALTER TABLE posts ADD COLUMN created_at_tz VARCHAR;",
    )
    .execute(db)
    .await?;
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
            .execute(db)
        })
    {
        fut.await?;
    }
    sqlx::query(
        "ALTER TABLE posts DROP COLUMN created_at;\
         ALTER TABLE posts RENAME COLUMN created_at_timestamp TO created_at;",
    )
    .execute(db)
    .await?;

    Ok(())
}

async fn add_backedup_column(db: &SqlitePool) -> Result<()> {
    const ADD_BACKEDUP_COLUMN: &str =
        "ALTER TABLE users ADD COLUMN backedup BOOLEAN DEFAULT false;";
    sqlx::query(ADD_BACKEDUP_COLUMN).execute(db).await?;

    Ok(())
}
