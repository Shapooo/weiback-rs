#![allow(unused)]
use std::collections::HashSet;
use std::hash::Hash;

use anyhow;
use bytes::Bytes;
use lazy_static::lazy_static;
use tera::{Context, Tera};

use crate::data::{Post, Posts};
use crate::persister::Persister;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("res/templates/*.html") {
            Ok(t) => t,
            Err(e) => panic!("tera template parse err: {e}"),
        };
        tera.autoescape_on(Vec::new());
        tera
    };
}

#[derive(Debug, Clone)]
pub struct HTMLGenerator();

impl HTMLGenerator {
    pub fn new() -> Self {
        Self()
    }

    pub fn generate_post(&self, mut post: Post) -> anyhow::Result<String> {
        let mut context = Context::new();
        context.insert("post", &post);
        let html = TEMPLATES.render("post.html", &context)?;

        Ok(html)
    }

    pub fn generate_posts(&self, posts: Posts) -> anyhow::Result<String> {
        let mut context = Context::new();
        context.insert("posts", &posts.data);
        let html = TEMPLATES.render("posts.html", &context)?;
        Ok(html)
    }

    pub fn generate_page(&self, posts: &str) -> anyhow::Result<String> {
        let mut context = Context::new();
        context.insert("html", &posts);
        let html = TEMPLATES.render("page.html", &context).unwrap();
        Ok(html)
    }
}

#[cfg(test)]
mod generator_test {
    use super::HTMLGenerator;
    use crate::data::{Post, Posts};
    use serde_json::from_str;

    #[tokio::test]
    async fn generate_post() {
        let s = include_str!("../res/one.json");
        let post = from_str::<Post>(s).unwrap();
        let gen = HTMLGenerator::new();
        let s = gen.generate_post(post).unwrap();
        println!("{}", s);
    }

    #[tokio::test]
    async fn generate_posts() {
        let s = include_str!("../res/full.json");
        let posts = from_str::<Posts>(s).unwrap();
        let gen = HTMLGenerator::new();
        let s = gen.generate_posts(posts).unwrap();
        println!("{}", s);
    }

    #[tokio::test]
    async fn generate_page() {
        let s = include_str!("../res/one.json");
        let post = from_str::<Post>(s).unwrap();
        let gen = HTMLGenerator::new();
        let s = gen.generate_post(post).unwrap();
        let s = gen.generate_page(&s).unwrap();
        println!("{}", s);
    }
}
