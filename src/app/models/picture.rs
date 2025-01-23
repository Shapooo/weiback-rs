use bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PictureType {
    InPost { url: String, post_id: i64 },
    Avatar { url: String, user_id: i64 },
    Emoji { url: String },
    Temporary { url: String },
}

pub struct PictureBlob(Bytes);

#[derive(Debug, Clone)]
pub struct Picture {
    pub type_: PictureType,
    pub blob: Option<PictureBlob>,
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
