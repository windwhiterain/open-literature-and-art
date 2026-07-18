mod build;
mod config;
mod gen;
mod project;
mod submit;
mod sync;
mod zola;

use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(name = "soil", about = "Open-literature-and-art project management tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize: install Zola and clone/pull the project repo
    Init,

    /// Sync author's working directory into the project
    Gen {
        /// Author's md working directory (defaults to current directory)
        #[arg(short, long)]
        source: Option<String>,
    },

    /// Sync missing .md / .meta.toml / meta.toml from project back to working directory
    Sync {
        /// Author's md working directory (defaults to current directory)
        #[arg(short, long)]
        source: Option<String>,
    },

    /// Build the site (gen + zola build)
    Build {
        /// Author's md working directory for gen step (optional)
        #[arg(short, long)]
        source: Option<String>,
    },

    /// Serve for local preview (gen + zola serve)
    Serve {
        /// Author's md working directory for gen step (optional)
        #[arg(short, long)]
        source: Option<String>,
    },

    /// Submit changes: gen + build + commit + push + create PR
    Submit {
        /// Author's md working directory for gen step (optional)
        #[arg(short, long)]
        source: Option<String>,

        /// Custom commit message
        #[arg(short, long)]
        message: Option<String>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => {
            let config = config::load()?;
            zola::ensure_zola()?;
            let root = project::project_root(&config.github_account)?;
            project::git_pull(&root)?;
            println!("Project ready at {}", root.display());
        }
        Command::Gen { source } => {
            let config = ensure_initialized()?;
            let source_path = source
                .as_deref()
                .map(Path::new)
                .unwrap_or_else(|| Path::new("."));
            let root = project::project_root(&config.github_account)?;
            gen::run(source_path, &config.author, &root)?;
        }
        Command::Sync { source } => {
            let config = ensure_initialized()?;
            let source_path = source
                .as_deref()
                .map(Path::new)
                .unwrap_or_else(|| Path::new("."));
            let root = project::project_root(&config.github_account)?;
            sync::run(source_path, &config.author, &root)?;
        }
        Command::Build { source } => {
            let config = ensure_initialized()?;
            let root = project::project_root(&config.github_account)?;
            if let Some(source) = source {
                gen::run(Path::new(&source), &config.author, &root)?;
            }
            build::run(&root)?;
        }
        Command::Serve { source } => {
            let config = ensure_initialized()?;
            let root = project::project_root(&config.github_account)?;
            if let Some(source) = source {
                gen::run(Path::new(&source), &config.author, &root)?;
            }
            build::serve(&root)?;
        }
        Command::Submit { source, message } => {
            let config = ensure_initialized()?;
            let root = project::project_root(&config.github_account)?;
            submit::run(
                source.as_deref(),
                &config.author,
                &config.github_account,
                message.as_deref(),
                &root,
            )?;
        }
    }

    Ok(())
}

fn ensure_initialized() -> anyhow::Result<config::Config> {
    if config::is_initialized() {
        return config::load();
    }
    println!("soil has not been initialized. Running init first...");
    let config = config::load()?;
    zola::ensure_zola()?;
    let root = project::project_root(&config.github_account)?;
    project::git_pull(&root)?;
    println!("Init complete. Continuing...");
    Ok(config)
}
