use clap::CommandFactory;
use clap_mangen::Man;
use std::env;
use std::fs::File;
use std::path::Path;
use std::process::Command;

// Import the clap::Parser struct from your main application
include!("src/cli.rs");

fn main() -> std::io::Result<()> {
    // --- Part 1: Generate man.1 from clap ---
    let cmd = Args::command();
    let out_dir = env::var_os("OUT_DIR").ok_or(std::io::ErrorKind::NotFound)?;
    let man1_path = Path::new(&out_dir).join("hyprland-minimizer.1");
    let mut man1_file = File::create(&man1_path)?;

    Man::new(cmd).render(&mut man1_file)?;
    println!("cargo:info=man page (1) generated at: {:?}", man1_path);

    // --- Part 2: Generate man.5 from Markdown using pandoc ---
    let man5_path = Path::new(&out_dir).join("hyprland-minimizer.5");
    let markdown_input = Path::new("doc/hyprland-minimizer.5.md");

    // Tell Cargo to re-run this script if the Markdown file changes.
    println!("cargo:rerun-if-changed={}", markdown_input.display());

    let status = Command::new("pandoc")
        .arg("-s") // Standalone file
        .arg("-t") // Target format
        .arg("man") // man page format (roff)
        .arg(markdown_input) // Input file
        .arg("-o") // Output file
        .arg(&man5_path)
        .status()?;

    if !status.success() {
        // Panic if pandoc fails. This will stop the build.
        panic!(
            "pandoc failed to generate man page with exit code: {:?}",
            status.code()
        );
    }

    println!("cargo:info=man page (5) generated at: {:?}", man5_path);

    Ok(())
}
