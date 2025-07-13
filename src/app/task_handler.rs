use std::ops::RangeInclusive;
use std::time::Duration;

use anyhow::Result;
use log::{error, info};
use tokio::{self, sync::mpsc::Sender, time::sleep};
use weibosdk_rs::{Post, WeiboAPI};

use super::search_args::SearchArgs;
use crate::models::Picture;
use crate::ports::{Exporter, Processer, Service, Storage, TaskResponse};

const SAVING_PERIOD: usize = 200;
const BACKUP_TASK_INTERVAL: Duration = Duration::from_secs(3);
const OTHER_TASK_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub struct TaskHandler<W: WeiboAPI, S: Storage, E: Exporter, P: Processer> {
    network: Option<W>,
    storage: S,
    exporter: E,
    processer: P,
    task_status_sender: Sender<TaskResponse>,
    uid: i64,
}

impl<W: WeiboAPI, S: Storage, E: Exporter, P: Processer> TaskHandler<W, S, E, P> {
    pub fn new(
        // mut login_info: LoginInfo,
        network: W,
        storage: S,
        exporter: E,
        processer: P,
        task_status_sender: Sender<TaskResponse>,
        uid: i64,
    ) -> Result<Self> {
        Ok(TaskHandler {
            network: Some(network),
            storage,
            exporter,
            processer,
            task_status_sender,
            uid,
        })
    }

    // initialize database、get emoticon data
    // pub async fn init(&mut self) {
    //     let res = self._init().await;
    //     self.handle_short_task_res(res).await;
    // }

    // async fn _init(&mut self) -> Result<TaskResponse> {
    //     init_emoticon(&self.network).await?;
    //     self.storage.init().await?;
    //     let (web_total, db_total) = tokio::join!(self.get_web_total_num(), self.get_db_total_num());
    //     let web_total = web_total?;
    //     debug!("initing...");
    //     Ok(TaskResponse::SumOfFavDB(web_total, db_total?))
    // }

    async fn _unfavorite_posts(&self) -> Result<()> {
        // let mut trans = self.storage.db().unwrap().acquire().await?;
        let ids = self.storage.get_posts_id_to_unfavorite().await?;
        let len = ids.len();
        for (i, id) in ids.into_iter().enumerate() {
            self.network.as_ref().unwrap().favorites_destroy(id).await?;
            info!("post {id} unfavorited");
            tokio::time::sleep(OTHER_TASK_INTERVAL).await;
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

    // backup self posts
    // pub async fn backup_self(&self, with_pic: bool, image_definition: u8) {
    //     self.backup_user(self.uid, with_pic, image_definition).await
    // }

    async fn _backup_user(&self, uid: i64, with_pic: bool, image_definition: u8) -> Result<()> {
        info!("download user {uid} posts");
        let search_args_vec = [
            SearchArgs::new().with_ori().with_text(),
            SearchArgs::new().with_ori().with_pic(),
            SearchArgs::new().with_ori().with_video(),
            SearchArgs::new().with_ori().with_music(),
            SearchArgs::new().with_ret().with_pic(),
            SearchArgs::new().with_ret().with_video(),
            SearchArgs::new().with_ret().with_music(),
        ];
        let total_category_num = search_args_vec.len();
        let one_category_ratio = 1.0 / total_category_num as f32;
        let mut total_page = 1;

        for (i, search_args) in search_args_vec.iter().enumerate() {
            let mut page = 1;
            loop {
                let len = self
                    .backup_one_page(uid, page, search_args, with_pic, image_definition)
                    .await?;
                info!("fetched {} posts in {}th page", len, page);
                if len == 0 {
                    break;
                }
                self.task_status_sender
                    .send(TaskResponse::InProgress(
                        (i as f32 + total_page as f32 / 100.) * one_category_ratio,
                        "备份中...耐心等待，干点别的...".into(),
                    ))
                    .await?;
                total_page += 1;
                page += 1;
                sleep(BACKUP_TASK_INTERVAL).await;
            }
        }
        Ok(())
    }

    // backup one page of posts of the user
    pub async fn backup_one_page(
        &self,
        uid: i64,
        page: u32,
        search_args: &SearchArgs,
        with_pic: bool,
        image_definition: u8,
    ) -> Result<usize> {
        let posts = self
            .network
            .as_ref()
            .unwrap()
            .profile_statuses(uid, page)
            .await?;
        let result = posts.len();
        for post in posts.iter() {
            self.storage.save_post(post).await?;
        }

        Ok(result)
    }

    async fn _export_from_local(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
        image_definition: u8,
    ) -> Result<()> {
        let task_name = format!("weiback-{}", chrono::Local::now().format("%F-%H-%M"));
        let target_dir = std::env::current_dir()?.join(task_name);

        let local_posts = self.load_fav_posts_from_db(range, reverse).await?;
        let posts_sum = local_posts.len();
        info!("fetched {} posts from local", posts_sum);

        let mut index = 1;
        loop {
            let subtask_name = format!("weiback-{index}");
            if local_posts.len() < SAVING_PERIOD {
                let html = self.processer.generate_html().await?;
                self.exporter
                    .export_page(&subtask_name, &html, &target_dir)
                    .await?;
                break;
            } else {
                let html = self.processer.generate_html().await?;
                self.exporter
                    .export_page(&subtask_name, &html, &target_dir)
                    .await?;
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
        let last_page = range.end() - 1;
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
            if i != last_page as usize {
                sleep(BACKUP_TASK_INTERVAL).await;
            }
        }
        info!("fetched {total_downloaded} posts in total");
        Ok(())
    }

    // handle short task result, like get_user_meta, which are tasks that take short time and
    // and return products to ui
    // different from handle_long_task_res, this function will not send TaskResponse::Finished
    // to ui, ui will not show task status info about this task
    async fn handle_short_task_res(&self, result: Result<TaskResponse>) {
        match result {
            Ok(res) => {
                info!("task finished");
                self.task_status_sender.send(res).await.unwrap();
            }
            Err(err) => {
                error!("{err:?}");
            }
        }
    }

    // handle long task result, like backup_favorites, which are tasks that take long time and
    // usually request by user, need to send task status info to ui for showing to user
    async fn handle_long_task_res(&self, result: Result<()>) {
        let mut db_total = 0;
        let mut web_total = 0;
        let result = self
            ._handle_long_task_res(result, &mut web_total, &mut db_total)
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

    async fn _handle_long_task_res(
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

    // backup one page of favorites
    pub async fn backup_one_fav_page(
        &self,
        uid: i64,
        page: u32,
        with_pic: bool,
        image_definition: u8,
    ) -> Result<usize> {
        // let posts = Post::fetch_fav_posts(uid, page, &self.network).await?;
        let posts = self.network.as_ref().unwrap().favorites(page).await?;
        let result = posts.len();
        let ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
        for post in posts.iter() {
            self.storage.save_post(post).await?;
        }

        // call mark_user_backed_up after all posts inserted, to ensure the post is in db
        for id in ids {
            self.storage.mark_post_favorited(id).await?;
        }

        Ok(result)
    }

    pub async fn load_fav_posts_from_db(
        &self,
        range: RangeInclusive<u32>,
        reverse: bool,
    ) -> Result<Vec<Post>> {
        let limit = (range.end() - range.start()) + 1;
        let offset = *range.start() - 1;
        self.storage.get_posts(limit, offset, reverse).await
    }

    // get total number of favorites in local database
    async fn get_db_total_num(&self) -> Result<u32> {
        self.storage.get_favorited_sum().await
    }
}

impl<W: WeiboAPI, S: Storage, E: Exporter, P: Processer> Service for TaskHandler<W, S, E, P> {
    // unfavorite all posts that are in weibo favorites
    async fn unfavorite_posts(&self) {
        self.handle_long_task_res(self._unfavorite_posts().await)
            .await
    }

    // backup user posts
    async fn backup_user(&self, uid: i64, with_pic: bool, image_definition: u8) {
        self.handle_long_task_res(self._backup_user(uid, with_pic, image_definition).await)
            .await
    }

    // export favorite posts from local database
    async fn export_from_local(
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

    // export favorite posts from weibo
    async fn backup_favorites(
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
}
