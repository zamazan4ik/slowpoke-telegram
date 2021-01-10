use url::{Url, ParseError};

pub fn is_url(text: &str) -> bool {
    Url::parse(text).is_ok()
}
