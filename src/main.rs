mod cli;
mod commands;
mod error;
mod ffmpeg;
pub mod silence;
mod temp;
mod ui;
mod vad;

use clap::{CommandFactory, Parser};
use owo_colors::OwoColorize;

use cli::{Cli, Commands};
use error::{AppError, format_error};
use temp::TempFileRegistry;

fn main() {
    let cli = Cli::parse();

    let registry = TempFileRegistry::new();
    temp::register_cleanup(registry.clone_inner());

    let result = match cli.command {
        None => {
            let _ = Cli::command().print_help();
            std::process::exit(0);
        }
        Some(Commands::Cut(args)) => commands::cut::run(args, cli.verbose, &registry),
        Some(Commands::Caption(args)) => commands::caption::run(args, cli.verbose, &registry),
        Some(Commands::Overlay(args)) => commands::overlay::run(args, cli.verbose, &registry),
        Some(Commands::Doctor(args)) => {
            let exit_code = commands::doctor::run(args.strict);
            std::process::exit(exit_code);
        }
        Some(Commands::Pipeline(args)) => commands::pipeline::run(args, cli.verbose, &registry),
    };

    if let Err(err) = result {
        if let Some(app_err) = err.downcast_ref::<AppError>() {
            eprintln!("{}", format_error(app_err));
        } else {
            eprintln!("{} {}", "error:".red().bold(), err);
        }
        std::process::exit(1);
    }
}
