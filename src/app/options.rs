use std::ops::RangeInclusive;

use crate::models::PictureDefinition;

#[derive(Debug, Clone, Copy, Default)]
pub enum UserPostFilter {
    #[default]
    All,
    Original,
    Video,
    Picture,
}

#[derive(Debug, Clone)]
pub struct TaskOptions {
    pub with_pic: bool,
    pub post_id: i64,
    pub uid: i64,
    pub pic_quality: PictureDefinition,
    pub reverse: bool,
    pub range: Option<RangeInclusive<u32>>,
    pub post_filter: UserPostFilter,
}

impl Default for TaskOptions {
    fn default() -> Self {
        Self {
            with_pic: false,
            post_id: 0,
            uid: 0,
            pic_quality: PictureDefinition::default(),
            reverse: false,
            range: None,
            post_filter: UserPostFilter::default(),
        }
    }
}

impl TaskOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pic(mut self, with_pic: bool) -> Self {
        self.with_pic = with_pic;
        self
    }

    pub fn with_user(mut self, uid: i64) -> Self {
        self.uid = uid;
        self
    }

    pub fn with_post(mut self, post_id: i64) -> Self {
        self.post_id = post_id;
        self
    }

    pub fn pic_quality(mut self, quality: PictureDefinition) -> Self {
        self.pic_quality = quality;
        self
    }

    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    pub fn range(mut self, range: RangeInclusive<u32>) -> Self {
        self.range = Some(range);
        self
    }

    pub fn post_filter(mut self, filter: UserPostFilter) -> Self {
        self.post_filter = filter;
        self
    }
}
