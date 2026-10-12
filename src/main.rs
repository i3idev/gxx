//! Main entry point.

mod cli;
mod cmd;
mod db;
mod error;
mod model;
mod render;
mod run;
mod tts;

use crate::error::GxxError;
use crate::run::run_cli;
use clap::Parser;
use cli::Cli;

fn main() {
    // Ensure stdout encoding on Windows
    #[cfg(windows)]
    {
        use std::io::{stdout, Write};
        let _ = stdout().flush();
    }

    let cli = Cli::parse();

    let result = run_cli(cli);

    match result {
        Ok(()) => {}
        Err(e) => {
            match e {
                GxxError::Msg(msg) => eprintln!("error: {}", msg),
                _ => eprintln!("error: {}", e),
            }
            std::process::exit(1);
        }
    }
}
