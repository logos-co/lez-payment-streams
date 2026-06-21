use std::{collections::HashMap, env, path::PathBuf};

use risc0_build::{embed_methods_with_options, DockerOptionsBuilder, GuestOptionsBuilder};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("methods crate should live under workspace root")
        .to_path_buf();

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
