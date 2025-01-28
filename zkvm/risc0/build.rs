use risc0_build::{embed_methods_with_options, DockerOptions, GuestOptions};
use std::{collections::HashMap, env, fs::File, io::Write};

fn main() {
    println!("cargo:rerun-if-env-changed=LCP_RISC0_BUILD");
    match env::var("LCP_RISC0_BUILD") {
        Ok(v) if v == "1" => {
            println!("debug: LCP_RISC0_BUILD is set");
        }
        _ => {
            println!("debug: LCP_RISC0_BUILD is not set");
            return;
        }
    }

    // Builds can be made deterministic, and thereby reproducible, by using Docker to build the
    // guest.
    let use_docker = Some(DockerOptions {
        root_dir: Some("../../".into()),
    });

    // Generate Rust source files for the methods crate.
    let guests = embed_methods_with_options(HashMap::from([(
        "guests",
        GuestOptions {
            features: Vec::new(),
            use_docker,
        },
    )]));

    if guests.len() != 1 {
        panic!("expected exactly one guest, found {}", guests.len());
    }
    let guest = guests[0].clone();
    println!("debug: path={} image_id={:?}", guest.path, guest.image_id);
    let elf_value = guest.elf;
    let image_id = guest.image_id;
    let mut elf_file = File::create("./artifacts/dcap-quote-verifier").unwrap();
    elf_file.write_all(&elf_value).unwrap();
    let mut methods_file = File::create("./src/methods.rs").unwrap();
    methods_file
        .write_all(
            format!(
                r##"
pub const DCAP_QUOTE_VERIFIER_ID: [u32; 8] = {image_id:?};
pub const DCAP_QUOTE_VERIFIER_ELF: &[u8] = include_bytes!("../artifacts/dcap-quote-verifier");
"##
            )
            .as_bytes(),
        )
        .unwrap();
}
