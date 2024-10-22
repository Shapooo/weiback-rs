use crate::network::WebFetcher;

use std::ops::DerefMut;

use anyhow::Result;
use bytes::Bytes;
use log::{debug, error, trace};
use sqlx::{Executor, FromRow, Sqlite};

const PIC_TYPE_AVATAR: u8 = 0;
const PIC_TYPE_INPOST: u8 = 1;
const PIC_TYPE_EMOJI: u8 = 2;
const PIC_TYPE_TMP: u8 = u8::MAX;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Picture {
    InPost(String, i64),
    Avatar(String, i64),
    Emoji(String),
    Tmp(String),
}

impl Picture {
    pub fn in_post(url: &str, post_id: i64) -> Self {
        let url = strip_url_queries(url).into();
        Self::InPost(url, post_id)
    }

    pub fn avatar(url: &str, uid: i64) -> Self {
        let url = strip_url_queries(url).into();
        Self::Avatar(url, uid)
    }

    pub fn emoji(url: &str) -> Self {
        let url = strip_url_queries(url).into();
        Self::Emoji(url)
    }

    pub fn tmp(url: &str) -> Self {
        let url = strip_url_queries(url).into();
        Self::Tmp(url)
    }

    pub fn is_tmp(&self) -> bool {
        matches!(self, Picture::Tmp(_))
    }

    pub fn get_url(&self) -> &str {
        match self {
            Picture::InPost(url, _) => url,
            Picture::Avatar(url, _) => url,
            Picture::Emoji(url) => url,
            Picture::Tmp(url) => url,
        }
    }

    #[allow(unused)]
    pub fn get_id(&self) -> &str {
        pic_url_to_id(self.get_url())
    }

    pub fn get_file_name(&self) -> &str {
        pic_url_to_file(self.get_url())
    }

    pub async fn create_table<E>(mut executor: E) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        PictureInner::create_table(&mut *executor).await?;
        PictureBlob::create_table(&mut *executor).await?;
        Ok(())
    }

    pub async fn persist<E>(&self, executor: E, fetcher: &WebFetcher) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        self.get_blob(executor, fetcher).await?;
        Ok(())
    }

    pub async fn get_blob<E>(&self, mut executor: E, fetcher: &WebFetcher) -> Result<Option<Bytes>>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        match self.query_blob(&mut *executor).await? {
            Some(blob) => Ok(Some(blob)),
            None => {
                let blob = match self.fetch_blob(fetcher).await {
                    Ok(blob) => blob,
                    Err(err) => {
                        error!("fetch pic failed: {}", err);
                        return Ok(None);
                    }
                };
                let blob = PictureBlob::new(self.get_url(), blob);
                let inner = PictureInner::from(self);
                if !self.is_tmp() {
                    blob.insert(&mut *executor).await?;
                    inner.insert(&mut *executor).await?;
                }
                Ok(Some(blob.blob))
            }
        }
    }

    async fn fetch_blob(&self, fetcher: &WebFetcher) -> Result<Bytes> {
        let url = self.get_url();
        debug!("fetch pic, url: {}", url);
        let res = fetcher.get(url).await?;
        let res_bytes = res.bytes().await?;
        trace!("fetched pic size: {}", res_bytes.len());
        Ok(res_bytes)
    }

    async fn query_blob<E>(&self, mut executor: E) -> Result<Option<Bytes>>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        let url = self.get_url();
        debug!("query img: {url}");
        Ok(
            sqlx::query_as::<Sqlite, (Vec<u8>,)>("SELECT blob FROM picture_blob WHERE url = ?")
                .bind(url)
                .fetch_optional(&mut *executor)
                .await?
                .map(|blob| Bytes::from(blob.0)),
        )
    }
}

#[derive(Debug, Clone, FromRow)]
struct PictureInner {
    pub id: String,
    pub uid: Option<i64>,
    pub post_id: Option<i64>,
    #[sqlx(rename = "type")]
    pub type_: u8,
}

impl From<&Picture> for PictureInner {
    fn from(value: &Picture) -> Self {
        match value {
            Picture::InPost(url, id) => Self {
                id: pic_url_to_id(url).into(),
                post_id: Some(*id),
                uid: None,
                type_: PIC_TYPE_INPOST,
            },
            Picture::Avatar(url, id) => Self {
                id: pic_url_to_id(url).into(),
                post_id: None,
                uid: Some(*id),
                type_: PIC_TYPE_AVATAR,
            },
            Picture::Emoji(url) => Self {
                id: pic_url_to_id(url).into(),
                post_id: None,
                uid: None,
                type_: PIC_TYPE_EMOJI,
            },
            Picture::Tmp(url) => Self {
                id: pic_url_to_id(url).into(),
                post_id: None,
                uid: None,
                type_: PIC_TYPE_TMP,
            },
        }
    }
}

impl PictureInner {
    pub async fn insert<E>(&self, mut executor: E) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        let result = sqlx::query("INSERT OR IGNORE INTO picture VALUES (?, ?, ?, ?)")
            .bind(&self.id)
            .bind(self.uid)
            .bind(self.post_id)
            .bind(self.type_)
            .execute(&mut *executor)
            .await?;
        trace!("insert picture result: {result:?}");
        Ok(())
    }

    pub async fn create_table<E>(mut executor: E) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS picture (\
            id TEXT PRIMARY KEY, \
            uid INTEGER, \
            post_id INTEGER, \
            type INTEGER);",
        )
        .execute(&mut *executor)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow)]
struct PictureBlob {
    pub url: String,
    pub id: String,
    pub blob: Bytes,
}

impl PictureBlob {
    pub fn new(url: &str, blob: Bytes) -> Self {
        Self {
            url: url.into(),
            id: pic_url_to_id(url).into(),
            blob,
        }
    }

    pub async fn insert<E>(&self, mut executor: E) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        let result = sqlx::query("INSERT OR IGNORE INTO picture_blob VALUES (?, ?, ?)")
            .bind(&self.url)
            .bind(&self.id)
            .bind(self.blob.as_ref())
            .execute(&mut *executor)
            .await?;
        trace!(
            "insert img blob {}-{}, result: {:?}",
            self.id,
            self.url,
            result
        );
        Ok(())
    }

    pub async fn create_table<E>(mut executor: E) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS picture_blob (\
            url TEXT PRIMARY KEY, \
            id TEXT, \
            blob BLOB);",
        )
        .execute(&mut *executor)
        .await?;
        Ok(())
    }
}

// TODO: handle exception
fn pic_url_to_file(url: &str) -> &str {
    url.rsplit('/')
        .next()
        .expect("it is not a valid picture url")
        .split('?')
        .next()
        .expect("it is not a valid picture url")
}

fn pic_url_to_id(url: &str) -> &str {
    let file = pic_url_to_file(url);
    let i = file.rfind('.').expect("it is not a valid picture url");
    &file[..i]
}

fn strip_url_queries(url: &str) -> &str {
    url.split('?').next().unwrap()
}

#[cfg(test)]
mod picture_tests {
    use super::*;
    #[test]
    fn pic_url_to_file_test() {
        let a = "https://baidu.com/hhhh.jpg?a=1&b=2";
        let res = pic_url_to_file(a);
        dbg!(res);
    }

    #[test]
    fn pic_url_to_id_test() {
        let a = "https://baidu.com/hhhh.jpg?a=1&b=2";
        let res = pic_url_to_id(a);
        dbg!(res);
    }
}
