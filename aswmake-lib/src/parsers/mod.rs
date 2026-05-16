pub mod loc;
pub mod bbs;

pub trait Parser {
    fn bms_filter() -> &'static str;
}

