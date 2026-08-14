use std::{collections::HashMap, env, path::PathBuf};

use risc0_build::{embed_methods_with_options, DockerOptionsBuilder, GuestOptionsBuilder};

fn workspace_root(start: PathBuf) -> PathBuf {
    let mut dir = start;
    loop {
        let cargo = dir.join("Cargo.toml");
        if cargo.is_file() {
            if let Ok(text) = std::fs::read_to_string(&cargo) {
                if text.contains("[workspace]") && text.contains("members") {
                    return dir;
                }
            }
        }
        if !dir.pop() {
            panic!("workspace Cargo.toml not found from methods crate");
        }
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = workspace_root(manifest_dir);

    let guest_opts = if env::var("RISC0_USE_DOCKER").ok().as_deref() == Some("1") {
        let docker = DockerOptionsBuilder::default()
            .root_dir(workspace_root)
            .build()
            .expect("docker options");
        GuestOptionsBuilder::default()
            .use_docker(docker)
            .build()
            .expect("guest options")
    } else {
        GuestOptionsBuilder::default()
            .build()
            .expect("guest options")
    };

    let mut opts = HashMap::new();
    opts.insert("lez_payment_streams-guest", guest_opts);

    embed_methods_with_options(opts);
}
