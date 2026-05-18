use std::env;
use std::path::PathBuf;
use std::process::{Command, Output};

fn decide_platform(command_str_no_ext: &PathBuf) -> Command {
    if cfg!(target_os = "windows") {
        return Command::new(command_str_no_ext.with_extension("exe"));
    }
    return Command::new(command_str_no_ext);
}

fn check_err(cmd_output: std::io::Result<Output>) -> anyhow::Result<String> {
    if let Ok(mut output) = cmd_output {
        output.stdout.extend_from_slice(&output.stderr);
        let cmd_out = String::from_utf8_lossy(&output.stdout);
    
        if let Some(code) = output.status.code() && code != 0 {
            return Err(anyhow::Error::msg(cmd_out.into_owned()));
        }
        
        return Ok(cmd_out.trim().to_string());
    }
    
    Err(cmd_output.err().map(|e| anyhow::Error::msg(e.to_string())).unwrap())
}

type ToolResult = anyhow::Result<String>;
macro_rules! declare_tool {
    ($name:ident) => {
        paste::paste! {
            #[allow(non_snake_case)]
            fn [ <__ $name _run> ](args: &[&std::path::Path]) -> ToolResult {
                static TOOL_PATH: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| {
                    let bytes = include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/",
                        env!(concat!(stringify!($name), "_EXE"))
                    ));
                    
                    let mut p = std::env::temp_dir();
                    p.push(stringify!($name).to_lowercase());

                    if !p.exists() {
                        std::fs::write(&p, bytes).expect(concat!("Could not write ", stringify!($name), "'s bin to temp file."));
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755));
                        }   
                    }
                    
                    p
                });
                
                let args = args.iter().filter(|a| !a.as_os_str().is_empty()).flat_map(|a| {
                    let a_str = a.to_str().unwrap();
                    if a_str.starts_with("-") || a_str.starts_with("--") {
                        return a_str.split_whitespace().map(|s| s.trim_matches('"').to_string()).collect::<Vec<_>>()
                    } 
                    vec![a_str.trim_matches('"').to_string()]
                });
                
                check_err(decide_platform(&*TOOL_PATH).args(args).output())
            }

            macro_rules! [<__ $name _macro_gen>] {
                ($dol:tt) => {
                    #[macro_export]
                    macro_rules! $name {
                        ($dol($dol arg:expr),*) => {
                            [<__ $name _run>](&[ $dol($dol arg.as_ref()),* ])
                        };
                    }
                };
            }
            
            [<__ $name _macro_gen>]!($);
        }
    };
}

pub mod bbspack;
pub mod bbscript;
pub mod repak;