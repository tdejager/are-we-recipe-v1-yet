use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Show detailed progress information
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze conda-forge feedstocks using cf-graph-countyfair data
    Analyze {
        /// Force re-clone the repository even if it exists
        #[arg(long)]
        force_clone: bool,
    },
}