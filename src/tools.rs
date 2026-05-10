use std::process::{Command, Output};

fn decide_platform(command_str_no_ext: &str) -> Command {
    if cfg!(target_os = "windows") {
        return Command::new(format!("{command_str_no_ext}.exe"))
    }
    return Command::new(command_str_no_ext);
}

fn check_result(result: std::io::Result<Output>) -> std::io::Result<Output> {
    match &result {
        Ok(o) => println!("\033[31m{}\033[0m", String::from_utf8_lossy(&o.stdout)),
        Err(e) => println!("\033[34m{e}\033[0m"),
    };
    result
}

pub fn bbspack<'a, I: IntoIterator<Item = &'a str>>(args: I) -> std::io::Result<Output> {
    check_result(decide_platform("./external/bbspack/target/release/bbspack").args(args).output())
}
pub fn bbscript(args: &[&str]) -> std::io::Result<Output> {
    check_result(decide_platform("./external/bbscript/target/release/bbscript").args(args).output())
}
pub fn u4pak(args: &[&str]) -> std::io::Result<Output> {
    check_result(decide_platform("./external/u4pak/target/release/u4pak").args(args).output())
}
pub fn quickbms(args: &[&str]) -> std::io::Result<Output> {
    check_result(decide_platform("./external/quickbms/quickbms_4gb_files").args(args).output())
}