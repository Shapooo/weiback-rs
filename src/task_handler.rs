use crate::{
    emoticon::init_emoticon, exporter::Exporter, login::LoginInfo, message::TaskResponse,
    persister::Persister, picture::Picture, post::Post, search_args::SearchArgs, user::User,
    web_fetcher::WebFetcher,
};

use std::{io::Cursor, ops::RangeInclusive, time::Duration};

use anyhow::{anyhow, Result};
use egui::{ColorImage, ImageData};
use image::io::Reader;
use log::{debug, error, info};
use tokio::{sync::mpsc::Sender, time::sleep};

const SAVING_PERIOD: usize = 200;

#[derive(Debug)]
pub struct TaskHandler {
    web_fetcher: WebFetcher,
    persister: Persister,
    task_status_sender: Sender<TaskResponse>,
    uid: i64,
}

impl TaskHandler {
    pub fn new(
        mut login_info: LoginInfo,
        task_status_sender: Sender<TaskResponse>,
    ) -> Result<Self> {
        let uid = if let serde_json::Value::Number(uid) = &login_info["uid"] {
            uid.as_i64().unwrap()
        } else {
            return Err(anyhow!("no uid field in login_info: {:?}", login_info));
        };
        let web_fetcher =
            WebFetcher::from_cookies(uid, serde_json::from_value(login_info["cookies"].take())?)?;
        let persister = Persister::new();

        Ok(TaskHandler {
            web_fetcher,
            persister,
            task_status_sender,
            uid,
        })
    }

    // initialize database、get emoticon data
    pub async fn init(&mut self) -> Result<()> {
        init_emoticon(&self.web_fetcher).await?;
        self.persister.init().await?;
        let (web_total, db_total) = tokio::join!(self.get_web_total_num(), self.get_db_total_num());
        let web_total = web_total?;
        debug!("initing...");
        self.task_status_sender
            .send(TaskResponse::SumOfFavDB(web_total, db_total?))
            .await?;
        Ok(())
    }

    pub async fn unfavorite_posts(&self) {
        self.handle_long_task_res(self._unfavorite_posts().await)
            .await
    }

    async fn _unfavorite_posts(&self) -> Result<()> {
        let mut trans = self.persister.db().unwrap().acquire().await?;
        let ids = Post::query_posts_to_unfavorite(trans.as_mut()).await?;
        let len = ids.len();
        for (i, id) in ids.into_iter().enumerate() {
            Post::unfavorite_post(id, trans.as_mut(), &self.web_fetcher).await?;
            info!("post {id} unfavorited");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let progress = i as f32 / len as f32;
            self.task_status_sender
                .send(TaskResponse::InProgress(
                    progress,
                    format!("已处理{i}条，共{len}条..."),
                ))
                .await?;
        }
        Ok(())
    }

    pub async fn backup_self(&self, with_pic: bool, image_definition: u8) {
        self.backup_user(self.uid, with_pic, image_definition).await
    }

    pub async fn backup_user(&self, uid: i64, with_pic: bool, image_definition: u8) {
        self.handle_long_task_res(self._backup_user(uid, with_pic, image_definition).await)
            .await
    }

    async fn _backup_user(&self, uid: i64, with_pic: bool, image_definition: u8) -> Result<()> {
        info!("download user {uid} posts");
        let mut page = 1;
        let search_args_vec = vec![
            SearchArgs::new().with_ori().with_text(),
            SearchArgs::new().with_ori().with_pic(),
            SearchArgs::new().with_ori().with_video(),
            SearchArgs::new().with_ori().with_music(),
            SearchArgs::new().with_ret().with_pic(),
            SearchArgs::new().with_ret().with_video(),
            SearchArgs::new().with_ret().with_music(),
        ];
        for search_args in search_args_vec {
            loop {
                let len = self
                    .backup_one_page(uid, page, &search_args, with_pic, image_definition)
                    .await?;
                if len == 0 {
                    break;
                }
                page += 1;
            }
        }
        let mut conn = self.persister.db().as_ref().unwrap().acquire().await?;
        User::mark_user_backed_up(uid, conn.as_mut()).await?;
        Ok(())
    }

    pub async fn backup_one_page(
        &self,
        uid: i64,
        page: u32,
        search_args: &SearchArgs,
        with_pic: bool,
        image_definition: u8,
    ) -> Result<usize> {
        let posts = Post::fetch_posts(uid, page, search_args, &self.web_fetcher).await?;
        let result = posts.len();
        Post::persist_posts(
            posts,
            with_pic,
            image_definition,
            self.persister.db().as_ref().unwrap(),
            &self.web_fetcher,
        )
        .await?;

        Ok(result)
    }

    pub async fn export_from_local(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
        image_definition: u8,
    ) {
        info!("fetch posts from local and export");
        self.handle_long_task_res(
            self._export_from_local(range, reverse, image_definition)
                .await,
        )
        .await;
    }

    async fn _export_from_local(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
        image_definition: u8,
    ) -> Result<()> {
        let task_name = format!("weiback-{}", chrono::Local::now().format("%F-%H-%M"));
        let target_dir = std::env::current_dir()?.join(task_name);

        let mut local_posts = self.load_fav_posts_from_db(range, reverse).await?;
        let posts_sum = local_posts.len();
        info!("fetched {} posts from local", posts_sum);

        let mut conn = self.persister.db().as_ref().unwrap().acquire().await?;
        let mut index = 1;
        loop {
            let subtask_name = format!("weiback-{index}");
            if local_posts.len() < SAVING_PERIOD {
                let html = Post::generate_html(
                    local_posts,
                    &subtask_name,
                    image_definition,
                    conn.as_mut(),
                    &self.web_fetcher,
                )
                .await?;
                Exporter::export_page(&subtask_name, html, &target_dir).await?;
                break;
            } else {
                let html = Post::generate_html(
                    local_posts.split_off(SAVING_PERIOD),
                    &subtask_name,
                    image_definition,
                    conn.as_mut(),
                    &self.web_fetcher,
                )
                .await?;
                Exporter::export_page(&subtask_name, html, &target_dir).await?;
            }
            let progress = (posts_sum - local_posts.len()) as f32 / posts_sum as f32;
            self.task_status_sender
                .send(TaskResponse::InProgress(
                    progress,
                    format!(
                        "已处理{}条，共{}条...\n可能需要下载图片",
                        posts_sum - local_posts.len(),
                        posts_sum
                    ),
                ))
                .await?;
            index += 1;
        }
        Ok(())
    }

    pub async fn backup_favorites(
        &self,
        range: RangeInclusive<u32>,
        with_pic: bool,
        image_definition: u8,
    ) {
        self.handle_long_task_res(
            self._backup_favorites(range, with_pic, image_definition)
                .await,
        )
        .await;
    }

    async fn _backup_favorites(
        &self,
        range: RangeInclusive<u32>,
        with_pic: bool,
        image_definition: u8,
    ) -> Result<()> {
        assert!(range.start() != &0);
        info!("favorites download range is {range:?}");
        let mut total_downloaded: usize = 0;
        let range = range.start() / 20 + 1..=range.end() / 20;
        let total_pages = (range.end() - range.start() + 1) as f32;

        for (i, page) in range.into_iter().enumerate() {
            let posts_sum = self
                .backup_one_fav_page(self.uid, page, with_pic, image_definition)
                .await?;
            total_downloaded += posts_sum;
            info!("fetched {} posts in {}th page", posts_sum, page);

            self.task_status_sender
                .send(TaskResponse::InProgress(
                    i as f32 / total_pages,
                    format!("已下载第{page}页...耐心等待，先干点别的"),
                ))
                .await?;
            sleep(Duration::from_secs(5)).await;
        }
        info!("fetched {total_downloaded} posts in total");
        Ok(())
    }

    pub async fn get_user_meta(&self, uid: i64) {
        self.handle_short_task_res(self._get_user_meta(uid).await)
            .await
    }

    async fn _get_user_meta(&self, uid: i64) -> Result<TaskResponse> {
        let user = User::fetch(uid, &self.web_fetcher).await?;
        let avatar = Picture::tmp(&user.profile_image_url);
        let mut conn = self.persister.db().as_ref().unwrap().acquire().await?;
        let avatar_blob = avatar
            .get_blob(conn.as_mut(), &self.web_fetcher)
            .await?
            .unwrap_or_default();
        let avatar_img = Reader::new(Cursor::new(avatar_blob))
            .with_guessed_format()?
            .decode()?
            .into_rgb8();
        let avatar_img = ColorImage::from_rgb(
            [avatar_img.width() as usize, avatar_img.height() as usize],
            &avatar_img.into_vec(),
        );
        Ok(TaskResponse::UserMeta(
            uid,
            user.screen_name,
            ImageData::Color(avatar_img.into()),
        ))
    }

    async fn handle_short_task_res(&self, result: Result<TaskResponse>) {
        match result {
            Ok(res) => {
                info!("task finished");
                self.task_status_sender.send(res).await.unwrap();
            }
            Err(err) => {
                error!("{err:?}");
                self.task_status_sender
                    .send(TaskResponse::Error(err))
                    .await
                    .unwrap();
            }
        }
    }

    async fn handle_long_task_res(&self, result: Result<()>) {
        let mut db_total = 0;
        let mut web_total = 0;
        let result = self
            ._handle_task_res(result, &mut web_total, &mut db_total)
            .await;
        match result {
            Err(err) => {
                error!("{err:?}");
                self.task_status_sender
                    .send(TaskResponse::Error(err))
                    .await
                    .unwrap();
            }
            Ok(()) => {
                info!("task finished");
                self.task_status_sender
                    .send(TaskResponse::Finished(web_total, db_total))
                    .await
                    .unwrap();
            }
        }
    }

    async fn _handle_task_res(
        &self,
        result: Result<()>,
        web_total: &mut u32,
        db_total: &mut u32,
    ) -> Result<()> {
        result?;
        let (web_total_res, db_total_res) =
            tokio::join!(self.get_web_total_num(), self.get_db_total_num());
        *web_total = web_total_res?;
        *db_total = db_total_res?;
        Ok(())
    }

    pub async fn backup_one_fav_page(
        &self,
        uid: i64,
        page: u32,
        with_pic: bool,
        image_definition: u8,
    ) -> Result<usize> {
        let posts = Post::fetch_fav_posts(uid, page, &self.web_fetcher).await?;
        let result = posts.len();
        let ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
        Post::persist_posts(
            posts,
            with_pic,
            image_definition,
            self.persister.db().as_ref().unwrap(),
            &self.web_fetcher,
        )
        .await?;

        // call mark_user_backed_up after all posts inserted, to ensure the post is in db
        let mut trans = self.persister.db().as_ref().unwrap().begin().await?;
        for id in ids {
            Post::mark_post_favorited(id, trans.as_mut()).await?;
        }
        trans.commit().await?;

        Ok(result)
    }

    pub async fn load_fav_posts_from_db(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
    ) -> Result<Vec<Post>> {
        let limit = (range.end() - range.start()) + 1;
        let offset = *range.start() - 1;
        let conn = self.persister.db().as_ref().unwrap().acquire().await?;
        Post::query_posts(limit, offset, reverse, conn).await
    }

    async fn get_web_total_num(&self) -> Result<u32> {
        self.web_fetcher.fetch_fav_total_num().await
    }

    async fn get_db_total_num(&self) -> Result<u32> {
        let conn = self.persister.db().as_ref().unwrap().acquire().await?;
        Post::query_favorited_sum(conn).await
    }
}
