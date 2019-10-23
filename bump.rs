use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const CRATES_TO_BUMP: &[&str] = &["cargo-wasi"];

const CRATES_TO_AVOID_BUMP: &[&str] = &["assemble"];

struct Crate {
    manifest: PathBuf,
    name: String,
    version: String,
    next_version: String,
}

fn main() {
    let mut crates = Vec::new();
    crates.push(read_crate("./Cargo.toml".as_ref()));
    find_crates("crates".as_ref(), &mut crates);

    let pos = CRATES_TO_BUMP
        .iter()
        .chain(CRATES_TO_AVOID_BUMP)
        .enumerate()
        .map(|(i, c)| (*c, i))
        .collect::<HashMap<_, _>>();
    crates.sort_by_key(|krate| pos.get(&krate.name[..]));

    for krate in crates.iter() {
        bump_version(&krate, &crates);
    }
}

fn find_crates(dir: &Path, dst: &mut Vec<Crate>) {
    if dir.join("Cargo.toml").exists() {
        let krate = read_crate(&dir.join("Cargo.toml"));
        if CRATES_TO_BUMP
            .iter()
            .chain(CRATES_TO_AVOID_BUMP)
            .any(|c| krate.name == *c)
        {
            dst.push(krate);
        } else {
            panic!("failed to find {:?} in whitelist or blacklist", krate.name);
        }
    }

    for entry in dir.read_dir().unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            find_crates(&entry.path(), dst);
        }
    }
}

fn read_crate(manifest: &Path) -> Crate {
    let mut name = None;
    let mut version = None;
    for line in fs::read_to_string(manifest).unwrap().lines() {
        if name.is_none() && line.starts_with("name = \"") {
            name = Some(
                line.replace("name = \"", "")
                    .replace("\"", "")
                    .trim()
                    .to_string(),
            );
        }
        if version.is_none() && line.starts_with("version = \"") {
            version = Some(
                line.replace("version = \"", "")
                    .replace("\"", "")
                    .trim()
                    .to_string(),
            );
        }
    }
    let name = name.unwrap();
    let version = version.unwrap();
    let next_version = if CRATES_TO_BUMP.contains(&&name[..]) {
        bump(&version)
    } else {
        version.clone()
    };
    Crate {
        manifest: manifest.to_path_buf(),
        name,
        version,
        next_version,
    }
}

fn bump_version(krate: &Crate, crates: &[Crate]) {
    let contents = fs::read_to_string(&krate.manifest).unwrap();

    let mut new_manifest = String::new();
    let mut is_deps = false;
    for line in contents.lines() {
        let mut rewritten = false;
        if line.starts_with("version =") {
            if CRATES_TO_BUMP.contains(&&krate.name[..]) {
                println!(
                    "bump `{}` {} => {}",
                    krate.name, krate.version, krate.next_version
                );
                new_manifest.push_str(&line.replace(&krate.version, &krate.next_version));
                rewritten = true;
            }
        }

        is_deps = if line.starts_with("[") {
            line.contains("dependencies")
        } else {
            is_deps
        };

        for other in crates {
            if !is_deps
                || (!line.starts_with(&format!("{} ", other.name))
                    && !line.starts_with("cargo-wasi-exe-"))
            {
                continue;
            }
            if !line.contains(&other.version) {
                if !line.contains("version =") {
                    continue;
                }
                panic!(
                    "{:?} has a dep on {} but doesn't list version {}",
                    krate.manifest, other.name, other.version
                );
            }
            rewritten = true;
            new_manifest.push_str(&line.replace(&other.version, &other.next_version));
            break;
        }
        if !rewritten {
            new_manifest.push_str(line);
        }
        new_manifest.push_str("\n");
    }
    fs::write(&krate.manifest, new_manifest).unwrap();
}

fn bump(version: &str) -> String {
    let mut iter = version.split('.').map(|s| s.parse::<u32>().unwrap());
    let major = iter.next().expect("major version");
    let minor = iter.next().expect("minor version");
    let patch = iter.next().expect("patch version");
    format!("{}.{}.{}", major, minor, patch + 1)
}
