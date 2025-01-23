use sqlx::{Executor, FromRow, Sqlite};

const PIC_TYPE_AVATAR: u8 = 0;
const PIC_TYPE_INPOST: u8 = 1;
const PIC_TYPE_EMOJI: u8 = 2;
const PIC_TYPE_TMP: u8 = u8::MAX;

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

    pub async fn persist<E>(&self, executor: E, fetcher: &NetworkImpl) -> Result<()>
    where
        E: DerefMut,
        for<'a> &'a mut E::Target: Executor<'a, Database = Sqlite>,
    {
        self.get_blob(executor, fetcher).await?;
        Ok(())
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
