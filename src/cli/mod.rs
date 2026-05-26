pub mod ops;
pub mod handlers;

use std::path::PathBuf;

use clap::*;


#[derive(Parser)]
pub struct Parser {
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
    
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    New,
    Ms { 
        #[command(subcommand)]
        command: MsCommands
    },
    Build { path: Option<String> },
}

#[derive(Subcommand)]
pub enum MsCommands {
    Add { game: String, pak_path: String},
    Remove { game: String },
    Update { game: String },
    Mount { game: String, mount_point: Option<String> },
}

impl MsCommands {
    pub fn validate_game(&self) -> (bool, &String) {
        let key = match self {
            MsCommands::Add { game, .. } => game,
            MsCommands::Remove { game } => game,
            MsCommands::Update { game } => game,
            MsCommands::Mount { game, .. } => game,
        };
        
        (key.parse::<aswmake_lib::TargetGame>().is_ok(), key)
    }
}