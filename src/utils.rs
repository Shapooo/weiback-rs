pub fn pic_url_to_file(url: &str) -> &str {
    url.rsplit('/')
        .next()
        .expect("it is not a valid picture url")
        .split('?')
        .next()
        .expect("it is not a valid picture url")
        .into()
}

pub fn pic_url_to_id(url: &str) -> &str {
    let file = pic_url_to_file(url);
    let i = file.rfind('.').expect("it is not a valid picture url");
    &file[..i]
}

pub fn strip_url_queries(url: &str) -> &str {
    url.split('?').next().unwrap()
}

use crate::error::{Error, Result};
use serde_json::Value;
pub fn value_as_str(v: &Value) -> Result<&str> {
    if let Some(s) = v.as_str() {
        Ok(s)
    } else {
        Err(Error::MalFormat(format!(
            "{} cannot convert to str",
            v.to_string(),
        )))
    }
}

#[cfg(test)]
mod utils_test {
    #[test]
    fn pic_url_to_file_test() {
        let a = "https://baidu.com/hhhh.jpg?a=1&b=2";
        let res = super::pic_url_to_file(a);
        dbg!(res);
    }

    #[test]
    fn pic_url_to_id_test() {
        let a = "https://baidu.com/hhhh.jpg?a=1&b=2";
        let res = super::pic_url_to_id(a);
        dbg!(res);
    }
}
