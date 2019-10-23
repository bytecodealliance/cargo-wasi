use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if let Ok(output) = Command::new("git").arg("rev-parse").arg("HEAD").output() {
        if output.status.success() {
            let sha = String::from_utf8(output.stdout).unwrap();

            let output = Command::new("git")
                .arg("show")
                .arg("-s")
                .arg("--format=%ci")
                .arg("HEAD")
                .output()
                .unwrap();
            let date = String::from_utf8(output.stdout).unwrap();

            println!(
                "cargo:rustc-env=GIT_INFO={} {}",
                &sha.trim()[0..10],
                date.split_whitespace().next().unwrap(),
            )
        }
    }
}
