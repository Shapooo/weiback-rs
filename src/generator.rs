use std::collections::HashSet;
use std::hash::Hash;

use anyhow;
use bytes::Bytes;
use tera;

use crate::data::Post;
use crate::fetcher::Fetcher;

#[derive(Debug, Clone)]
pub struct HTMLGenerator();

impl HTMLGenerator {
    pub fn new() -> Self {
        Self()
    }

    pub async fn generate_post<'a, P: Post<'a>>(
        &self,
        post: P,
        fetcher: &Fetcher,
    ) -> anyhow::Result<HTMLPosts> {
        todo!()
    }

    pub async fn generate_page(&self, posts: HTMLPosts) -> anyhow::Result<HTMLPage> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct HTMLPosts {
    pub html: String,
    pub pics: HashSet<Picture>,
}

impl HTMLPosts {
    pub fn new() -> Self {
        HTMLPosts {
            html: String::new(),
            pics: HashSet::new(),
        }
    }
    pub fn join(self, rhs: Self) -> Self {
        todo!()
    }
    pub fn merge(&mut self, rhs: Self) {
        self.html.push_str(&rhs.html);
        rhs.pics.into_iter().for_each(|p| {
            self.pics.insert(p);
        });
    }
}

impl Default for HTMLPosts {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct HTMLPage {
    pub html: String,
    pub pics: Vec<Picture>,
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub name: String,
    pub blob: Bytes,
}

impl PartialEq for Picture {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Picture {}

impl Hash for Picture {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
