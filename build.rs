// This attribute ensures the code below is only compiled when the feature is enabled.
#[cfg(feature = "generate-man-pages")]
mod man_generator {
    use clap::CommandFactory;
    use clap_mangen::Man;
    use std::env;
    use std::fs::File;
    use std::path::Path;
    use std::process::Command;

    // Import the clap::Parser struct from your main application
    include!("src/cli.rs");

    pub fn generate() -> std::io::Result<()> {
        let cmd = Args::command();
        let out_dir = env::var_os("OUT_DIR").ok_or(std::io::ErrorKind::NotFound)?;

        // --- Generate man.1 from clap ---
        let man1_path = Path::new(&out_dir).join("hyprland-minimizer.1");
        let mut man1_file = File::create(&man1_path)?;
        Man::new(cmd).render(&mut man1_file)?;
        println!("cargo:info=man page (1) generated at: {:?}", man1_path);

        // --- Generate man.5 from Markdown using pandoc ---
        let man5_path = Path::new(&out_dir).join("hyprland-minimizer.5");
        let markdown_input = Path::new("doc/hyprland-minimizer.5.md");
        println!("cargo:rerun-if-changed={}", markdown_input.display());

        let pandoc_status = Command::new("pandoc")
            .arg("-s")
            .arg("-t")
            .arg("man")
            .arg(markdown_input)
            .arg("-o")
            .arg(&man5_path)
            .status()?;

        if !pandoc_status.success() {
            panic!(
                "pandoc failed to generate man page with exit code: {:?}",
                pandoc_status.code()
            );
        }
        println!("cargo:info=man page (5) generated at: {:?}", man5_path);

        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    // If the feature is enabled, call the generator function.
    #[cfg(feature = "generate-man-pages")]
    man_generator::generate()?;

    // If the feature is not enabled, this main function does nothing.
    Ok(())
}
