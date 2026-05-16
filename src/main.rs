use aswmake_lib::error::InvalidFilePath;
use color_print::cprintln;

fn main() -> anyhow::Result<()> {
    cprintln!("<blue>Hello World<blue>");
    
    Err(InvalidFilePath("foo"))
}


#[cfg(test)]
use suitest::{suite, suite_cfg};
#[cfg(test)]
#[suite(main_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    #[test]
    fn foo() {
        assert!(true)
    }
}