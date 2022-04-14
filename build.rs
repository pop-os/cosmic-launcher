#[cfg(feature = "dev")]
use std::process::Command;

#[cfg(feature = "dev")]
fn main() {
    Command::new("mkdir")
        .args(["-p", "target/"])
        .output()
        .expect("failed to create target/");
    Command::new("glib-compile-resources")
        .args([
            "--sourcedir=data/resources",
            "--target=target/compiled.gresource",
            "data/resources/resources.gresource.xml",
        ])
        .output()
        .expect("failed to compile gresources");
}

#[cfg(not(feature = "dev"))]
pub fn main() {}
