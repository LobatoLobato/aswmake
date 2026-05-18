pub mod loc;
pub mod bbs;

pub trait Parser {
    fn ms_filter() -> &'static str;
}

