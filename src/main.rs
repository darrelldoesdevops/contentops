mod cli;
mod commands;
mod error;
mod ffmpeg;
pub mod silence;
mod temp;
mod tiktok;
mod ui;
mod vad;

use clap::{CommandFactory, Parser};
use owo_colors::OwoColorize;

use cli::{Cli, Commands};
use error::{AppError, format_error};
use temp::TempFileRegistry;

fn main() {
    let cli = Cli::parse();

    let env_vcodec = std::env::var("CONTENTOPS_VCODEC").ok();
    let env_vbitrate = std::env::var("CONTENTOPS_VBITRATE").ok();
    let process_config = match ffmpeg::resolve_process_config(
        cli.vcodec,
        cli.vbitrate.as_deref(),
        env_vcodec.as_deref(),
        env_vbitrate.as_deref(),
    ) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{} {}", "error:".red().bold(), error);
            std::process::exit(2);
        }
    };
    ffmpeg::install_process_config(process_config)
        .expect("process FFmpeg configuration should only be initialized once");

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
