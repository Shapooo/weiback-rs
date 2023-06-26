use anyhow;
use lazy_static::lazy_static;
use log::debug;
use tera::{Context, Tera};

use crate::data::{Post, Posts};

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut path = std::env::current_exe().unwrap();
        path.pop();
        let mut path = path.into_os_string().into_string().unwrap();
        path.push_str("/res/templates/*.html");
        debug!("init tera from template: {}", path);
        let mut tera = match Tera::new(&path) {
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

    #[allow(unused)]
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
