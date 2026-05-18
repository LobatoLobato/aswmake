use clap::Parser as _;

mod cli;
mod error;
mod cfg;

fn main() -> anyhow::Result<()> {
    let mut cfg = cfg::ToolConfig::load()?;
    let args = cli::Parser::parse();
    
    match &args.command {
        Some(cli::Commands::New) => cli::handlers::new(&mut cfg),
        Some(cli::Commands::Ms { command }) => cli::handlers::ms(&mut cfg, command),
        Some(cli::Commands::Build { path }) => cli::handlers::build(path.as_ref().unwrap_or(&String::from("."))),
        None => {
            println!("There was no subcommand given");
            Ok(())
        }
    }
}

#[cfg(test)]
use suitest::{suite, suite_cfg};

#[cfg(test)]
#[suite(main_rs)]
#[suite_cfg(sequential = true, verbose = false)]
mod tests {
    #[test]
    fn foo() {
        assert!(true);
    }
}
