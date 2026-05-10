use std::env;
use std::path::PathBuf;
use std::process::{Command, Output};

fn decide_platform(command_str_no_ext: &PathBuf) -> Command {
    if cfg!(target_os = "windows") {
        return Command::new(command_str_no_ext.join(".exe"));
    }
    return Command::new(command_str_no_ext);
}

fn check_result(result: std::io::Result<Output>) -> std::io::Result<Output> {
    match &result {
        Ok(o) => println!(
            "\x1b[31m{}{}\x1b[0m",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => println!("\x1b[34m{e}\x1b[0m"),
    };
    result
}

type ToolFn = Box<dyn Fn(&[&str]) -> std::io::Result<Output> + Send + Sync>;
macro_rules! declare_tool {
    ($name:ident) => {
        pub static $name: std::sync::LazyLock<ToolFn> = std::sync::LazyLock::new(|| {
            let bytes = include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/",
                env!(concat!(stringify!($name), "_EXE"))
            ));
            let mut p = std::env::temp_dir();
            p.push(stringify!($name));

            if !p.exists() {
                std::fs::write(&p, bytes).unwrap();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
                }
            }

            Box::new(move |args: &[&str]| check_result(decide_platform(&p).args(args).output()))
        });
    };
}

declare_tool!(BBSPACK);
declare_tool!(BBSCRIPT);
declare_tool!(U4PAK);
declare_tool!(QUICKBMS);
