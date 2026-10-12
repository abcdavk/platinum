use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    let resources = Path::new("src/resources");
    let blp_files = ["window", "platinum-manager"];

    for name in blp_files {
        let input = resources.join(format!("{name}.blp"));
        let output = resources.join(format!("{name}.ui"));

        println!("cargo:rerun-if-changed={}", input.display());

        let result = Command::new("blueprint-compiler")
            .arg("compile")
            .arg(input)
            .arg("--output")
            .arg(output)
            .stdout(Stdio::null())
            .status()
            .expect("Failed to run blueprint-compiler");

        assert!(result.success(), "Blueprint compilation failed: {name}");
    }

    println!("cargo:rerun-if-changed=src/resources/resources.gresource.xml");

    glib_build_tools::compile_resources(
        &["src/resources"],
        "src/resources/resources.gresource.xml",
        "resources.gresource",
    );
}
