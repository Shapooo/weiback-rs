use lazy_static::lazy_static;
use log::{debug, error};
use tera::{Context, Tera};

use crate::data::{Post, Posts};
use crate::error::Result;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let path = std::env::current_exe().unwrap();
        let path = path
            .parent()
            .expect("the executable should have parent, maybe bugs in there");
        let mut path = path
            .to_str()
            .expect("template path cannot convert to str")
            .to_owned();
        path.push_str("/res/templates/*.html");
        debug!("init tera from template: {}", path);
        let mut tera = match Tera::new(&path) {
            Ok(t) => t,
            Err(e) => {
                error!("tera template parse err: {e}");
                panic!("tera template parse err: {e}")
            }
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
    pub fn generate_post(&self, mut post: Post) -> Result<String> {
        let mut context = Context::new();
        context.insert("post", &post);
        let html = TEMPLATES.render("post.html", &context)?;

        Ok(html)
    }

    pub fn generate_posts(&self, posts: Posts) -> Result<String> {
        let mut context = Context::new();
        context.insert("posts", &posts);
        let html = TEMPLATES.render("posts.html", &context)?;
        Ok(html)
    }

    pub fn generate_page(&self, posts: &str) -> Result<String> {
        let mut context = Context::new();
        context.insert("html", &posts);
        let html = TEMPLATES.render("page.html", &context)?;
        Ok(html)
    }
}
