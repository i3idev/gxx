//! Main entry point.

mod cmd;
mod db;
mod error;
mod model;
mod render;
mod run;

use crate::error::GxxError;
use crate::run::run_internal;

fn main() {
    // Ensure stdout encoding on Windows
    #[cfg(windows)]
    {
        use std::io::{stdout, Write};
        let _ = stdout().flush();
    }

    let mut args: Vec<String> = std::env::args().skip(1).collect();

    // Handle --db anywhere on the command line
    let mut db_override = None;
    if let Some(pos) = args.iter().position(|a| a == "--db") {
        if pos + 1 >= args.len() {
            eprintln!("error: --db needs a name");
            std::process::exit(1);
        }
        db_override = Some(args[pos + 1].clone());
        args.drain(pos..pos + 2);
    }

    let result = run_internal(args, db_override.as_deref());

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
