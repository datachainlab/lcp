use std::fs::{create_dir_all, remove_dir_all};
use std::path::{Path, PathBuf};
use std::process;

use argh::FromArgs;
use walkdir::WalkDir;

#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "compile")]
/// Compile
pub struct CompileCmd {
    #[argh(option, short = 'i')]
    /// path to the Cosmos IBC proto files
    ibc: PathBuf,

    #[argh(option, short = 'o')]
    /// path to output the generated Rust sources into
    out: PathBuf,

    #[argh(option, short = 'd')]
    /// path to output the proto descriptor into
    descriptor: PathBuf,
}

impl CompileCmd {
    pub fn run(&self) {
        Self::compile_protos(&self.ibc, self.out.as_ref(), &self.descriptor);
    }

    fn compile_protos(ibc_dir: &Path, out_dir: &Path, descriptor_path: &Path) {
        // Remove old compiled files
        remove_dir_all(&out_dir).unwrap_or_default();
        create_dir_all(&out_dir).unwrap();

        println!(
            "[info ] Compiling LCP .proto files to Rust into '{}'...",
            out_dir.display()
        );

        let root = env!("CARGO_MANIFEST_DIR");

        // Paths
        let proto_paths = [
            format!("{}/../proto/definitions", root),
            format!(
                "{}/proto/ibc/core/client/v1/client.proto",
                ibc_dir.display()
            ),
        ];

        let proto_includes_paths = [
            format!("{}/../proto/definitions", root),
            format!("{}/proto", ibc_dir.display()),
            format!("{}/third_party/proto", ibc_dir.display()),
        ];

        // List available proto files
        let mut protos: Vec<PathBuf> = vec![];
        for proto_path in &proto_paths {
            println!("Looking for proto files in {:?}", proto_path);
            protos.append(
                &mut WalkDir::new(proto_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_type().is_file()
                            && e.path().extension().is_some()
                            && e.path().extension().unwrap() == "proto"
                    })
                    .map(|e| e.into_path())
                    .collect(),
            );
        }

        println!("Found the following protos:");
        // Show which protos will be compiled
        for proto in &protos {
            println!("\t-> {:?}", proto);
        }
        println!("[info ] Compiling..");

        // List available paths for dependencies
        let includes: Vec<PathBuf> = proto_includes_paths.iter().map(PathBuf::from).collect();
        let attrs_serde = r#"#[derive(::serde::Serialize, ::serde::Deserialize)]"#;
        let attrs_jsonschema =
            r#"#[cfg_attr(feature = "json-schema", derive(::schemars::JsonSchema))]"#;
        let attrs_ord = "#[derive(Eq, PartialOrd, Ord)]";
        let attrs_serde_default = r#"#[serde(default)]"#;
        let compilation = tonic_build::configure()
            .build_client(true)
            .compile_well_known_types(true)
            .client_mod_attribute(".", r#"#[cfg(feature = "client")]"#)
            .server_mod_attribute(".", r#"#[cfg(feature = "server")]"#)
            .build_server(true)
            .out_dir(out_dir)
            .type_attribute(".google.protobuf.Any", attrs_serde)
            .type_attribute(".google.protobuf.Timestamp", attrs_serde)
            .type_attribute(".google.protobuf.Duration", attrs_serde)
            .type_attribute(".ibc.core.client.v1", attrs_serde)
            .type_attribute(".ibc.core.client.v1.Height", attrs_ord)
            .type_attribute(".ibc.core.client.v1.Height", attrs_jsonschema)
            .field_attribute(".ibc.core.client.v1.Height", attrs_serde_default)
            .type_attribute(".lcp.service.enclave.v1", attrs_serde)
            .type_attribute(".lcp.service.elc.v1", attrs_serde)
            .file_descriptor_set_path(descriptor_path)
            .compile(&protos, &includes);

        match compilation {
            Ok(_) => {
                println!("Successfully compiled proto files");
            }
            Err(e) => {
                println!("Failed to compile:{:?}", e.to_string());
                process::exit(1);
            }
        }
    }
}
