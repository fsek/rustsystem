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

    let release = std::env::var("PROFILE").as_deref() == Ok("release");

    if release {
        pnpm().arg("install").status()?;
        pnpm().arg("build").status()?;
    } else {
        println!(
            "cargo:warning=Frontend is not built in debug mode. Run `pnpm install && pnpm dev` in the frontend directory."
        );

        // generate the output directory
        std::fs::create_dir_all("frontend/dist")?;
    }

    Ok(())
}
