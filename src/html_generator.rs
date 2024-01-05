use crate::{error::Result, post::Post};

use lazy_static::lazy_static;
use log::{debug, error};
use serde_json::Value;
use tera::{Context, Tera};

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
        path.push_str("/templates/*.html");
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
    #[allow(unused)]
    pub fn generate_post(mut post: Post) -> Result<String> {
        let mut context = Context::new();
        context.insert("post", &post);
        let html = TEMPLATES.render("post.html", &context)?;

        Ok(html)
    }

    pub fn generate_posts(posts: Vec<Value>) -> Result<String> {
        let mut context = Context::new();
        context.insert("posts", &posts);
        let html = TEMPLATES.render("posts.html", &context)?;
        Ok(html)
    }

    pub fn generate_page(posts: &str) -> Result<String> {
        let mut context = Context::new();
        context.insert("html", &posts);
        let html = TEMPLATES.render("page.html", &context)?;
        Ok(html)
    }
}
