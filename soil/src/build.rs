use std::path::Path;
use crate::zola;

pub fn run(project_root: &Path) -> anyhow::Result<()> {
    zola::run(&["build"], project_root)?;
    println!("Site built at {}", project_root.join("public").display());
    Ok(())
}

pub fn serve(project_root: &Path) -> anyhow::Result<()> {
    zola::run(&["serve"], project_root)?;
    Ok(())
}
