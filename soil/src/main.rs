mod build;
mod config;
mod gen;
mod project;
mod submit;
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
    /// Install Zola and clone the project repo
    Install,

    /// Sync author's working directory into the project
    Gen {
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
        Command::Install => {
            let _config = config::load()?;
            zola::ensure_zola()?;
            let root = project::project_root()?;
            println!("Project ready at {}", root.display());
        }
        Command::Gen { source } => {
            let config = config::load()?;
            let source_path = source
                .as_deref()
                .map(Path::new)
                .unwrap_or_else(|| Path::new("."));
            let root = project::project_root()?;
            project::git_pull(&root)?;
            gen::run(source_path, &config.author, &root)?;
        }
        Command::Build { source } => {
            let config = config::load()?;
            let root = project::project_root()?;
            project::git_pull(&root)?;
            if let Some(source) = source {
                gen::run(Path::new(&source), &config.author, &root)?;
            }
            build::run(&root)?;
        }
        Command::Serve { source } => {
            let config = config::load()?;
            let root = project::project_root()?;
            project::git_pull(&root)?;
            if let Some(source) = source {
                gen::run(Path::new(&source), &config.author, &root)?;
            }
            build::serve(&root)?;
        }
        Command::Submit { source, message } => {
            let config = config::load()?;
            let root = project::project_root()?;
            project::git_pull(&root)?;
            submit::run(
                source.as_deref(),
                &config.author,
                &config.github_token,
                message.as_deref(),
                &root,
            )?;
        }
    }

    Ok(())
}
