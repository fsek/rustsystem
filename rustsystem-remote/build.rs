//! This build script bundles the frontend.

use std::process::Command;

fn pnpm() -> Command {
    let mut cmd = Command::new("pnpm");
    cmd.current_dir("frontend");
    cmd
}

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/package.json");

    pnpm().arg("install").status()?;
    pnpm().arg("build").status()?;

    Ok(())
}
