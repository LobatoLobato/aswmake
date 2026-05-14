pub mod loc;
pub mod error;

pub trait Parser {
    fn bms_filter() -> &'static str;
}

