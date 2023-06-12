use std::fs::remove_dir_all;
use std::fs::{copy, create_dir_all};
use std::path::{Path, PathBuf};
use std::process;

use tempfile::TempDir;
use walkdir::WalkDir;

use argh::FromArgs;
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
        let tmp_ibc = TempDir::new().unwrap();
        Self::compile_ibc_protos(&self.ibc, tmp_ibc.as_ref());

        let tmp_lcp = TempDir::new().unwrap();
        Self::compile_lcp_protos(&self.ibc, tmp_lcp.as_ref(), &self.descriptor);

        Self::copy_generated_files(tmp_lcp.as_ref(), tmp_ibc.as_ref(), &self.out);
    }

    fn compile_lcp_protos(ibc_dir: &Path, out_dir: &Path, descriptor_path: &Path) {
        println!(
            "[info ] Compiling LCP .proto files to Rust into '{}'...",
            out_dir.display()
        );

        let root = env!("CARGO_MANIFEST_DIR");

        // Paths
        let proto_paths = [
            format!(
                "{}/../proto/definitions",
                root
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
        let compilation = tonic_build::configure()
            .build_client(true)
            .compile_well_known_types(true)
            .client_mod_attribute(".", r#"#[cfg(feature = "client")]"#)
            .server_mod_attribute(".", r#"#[cfg(feature = "server")]"#)
            .build_server(true)
            .out_dir(out_dir)
            .extern_path(".tendermint", "::tendermint_proto")
            .type_attribute(".cosmos.upgrade.v1beta1", attrs_serde)
            .type_attribute(".cosmos.base.v1beta1", attrs_serde)
            .type_attribute(".cosmos.base.query.v1beta1", attrs_serde)
            .type_attribute(".cosmos.bank.v1beta1", attrs_serde)
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

    fn compile_ibc_protos(ibc_dir: &Path, out_dir: &Path) {
        println!(
            "[info ] Compiling IBC .proto files to Rust into '{}'...",
            out_dir.display()
        );

        // Paths
        let proto_paths = [
            // ibc-go proto files
            format!(
                "{}/proto/ibc/core/client/v1/client.proto",
                ibc_dir.display()
            ),
        ];

        let proto_includes_paths = [
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
        let attrs_eq = "#[derive(Eq)]";
        let attrs_serde_default = r#"#[serde(default)]"#;
        let attrs_serde_base64 = r#"#[serde(with = "crate::base64")]"#;
        let attrs_jsonschema_str =
            r#"#[cfg_attr(feature = "json-schema", schemars(with = "String"))]"#;

        let compilation = tonic_build::configure()
            .build_client(true)
            .compile_well_known_types(true)
            .client_mod_attribute(".", r#"#[cfg(feature = "client")]"#)
            .build_server(false)
            .out_dir(out_dir)
            .extern_path(".tendermint", "::tendermint_proto")
            .type_attribute(".google.protobuf.Any", attrs_serde)
            .type_attribute(".google.protobuf.Timestamp", attrs_serde)
            .type_attribute(".google.protobuf.Duration", attrs_serde)
            .type_attribute(".ibc.core.client.v1", attrs_serde)
            .type_attribute(".ibc.core.client.v1.Height", attrs_ord)
            .type_attribute(".ibc.core.client.v1.Height", attrs_jsonschema)
            .field_attribute(".ibc.core.client.v1.Height", attrs_serde_default)
            .type_attribute(".ibc.core.commitment.v1", attrs_serde)
            .type_attribute(".ibc.core.commitment.v1.MerkleRoot", attrs_jsonschema)
            .field_attribute(
                ".ibc.core.commitment.v1.MerkleRoot.hash",
                attrs_serde_base64,
            )
            .field_attribute(
                ".ibc.core.commitment.v1.MerkleRoot.hash",
                attrs_jsonschema_str,
            )
            .type_attribute(".ibc.core.commitment.v1.MerklePrefix", attrs_jsonschema)
            .field_attribute(
                ".ibc.core.commitment.v1.MerklePrefix.key_prefix",
                attrs_serde_base64,
            )
            .field_attribute(
                ".ibc.core.commitment.v1.MerklePrefix.key_prefix",
                attrs_jsonschema_str,
            )
            .type_attribute(".ibc.core.channel.v1", attrs_serde)
            .type_attribute(".ibc.core.channel.v1.Channel", attrs_jsonschema)
            .type_attribute(".ibc.core.channel.v1.Counterparty", attrs_jsonschema)
            .type_attribute(".ibc.core.connection.v1", attrs_serde)
            .type_attribute(".ibc.core.connection.v1.ConnectionEnd", attrs_jsonschema)
            .type_attribute(".ibc.core.connection.v1.Counterparty", attrs_jsonschema)
            .type_attribute(".ibc.core.connection.v1.Version", attrs_jsonschema)
            .type_attribute(".ibc.core.types.v1", attrs_serde)
            .type_attribute(".ibc.applications.transfer.v1", attrs_serde)
            .type_attribute(".ibc.applications.transfer.v2", attrs_serde)
            .type_attribute(
                ".ibc.applications.interchain_accounts.controller.v1",
                attrs_serde,
            )
            .type_attribute(".ics23", attrs_serde)
            .type_attribute(".ics23.LeafOp", attrs_eq)
            .type_attribute(".ics23.LeafOp", attrs_jsonschema)
            .field_attribute(".ics23.LeafOp.prehash_key", attrs_serde_default)
            .field_attribute(".ics23.LeafOp.prefix", attrs_serde_base64)
            .field_attribute(".ics23.LeafOp.prefix", attrs_jsonschema_str)
            .type_attribute(".ics23.InnerOp", attrs_jsonschema)
            .field_attribute(".ics23.InnerOp.prefix", attrs_serde_base64)
            .field_attribute(".ics23.InnerOp.prefix", attrs_jsonschema_str)
            .field_attribute(".ics23.InnerOp.suffix", attrs_serde_base64)
            .field_attribute(".ics23.InnerOp.suffix", attrs_jsonschema_str)
            .type_attribute(".ics23.InnerOp", attrs_eq)
            .type_attribute(".ics23.ProofSpec", attrs_eq)
            .type_attribute(".ics23.ProofSpec", attrs_jsonschema)
            .field_attribute(".ics23.ProofSpec.max_depth", attrs_serde_default)
            .field_attribute(".ics23.ProofSpec.min_depth", attrs_serde_default)
            .type_attribute(".ics23.InnerSpec", attrs_eq)
            .type_attribute(".ics23.InnerSpec", attrs_jsonschema)
            .field_attribute(".ics23.InnerSpec.empty_child", attrs_serde_default)
            .field_attribute(".ics23.InnerSpec.empty_child", attrs_serde_base64)
            .field_attribute(".ics23.InnerSpec.empty_child", attrs_jsonschema_str)
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

    fn copy_generated_files(from_dir_lcp: &Path, from_dir_ibc: &Path, to_dir: &Path) {
        println!(
            "[info ] Copying generated files into '{}'...",
            to_dir.display()
        );

        // Remove old compiled files
        remove_dir_all(&to_dir).unwrap_or_default();
        create_dir_all(&to_dir).unwrap();

        // Copy new compiled files (prost does not use folder structures)
        // Copy the SDK files first
        let errors_sdk = WalkDir::new(from_dir_lcp)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                copy(
                    e.path(),
                    format!(
                        "{}/{}",
                        to_dir.display(),
                        &e.file_name().to_os_string().to_str().unwrap()
                    ),
                )
            })
            .filter_map(|e| e.err())
            .collect::<Vec<_>>();

        if !errors_sdk.is_empty() {
            for e in errors_sdk {
                println!("[error] Error while copying SDK-compiled file: {}", e);
            }

            panic!("[error] Aborted.");
        }

        // Copy the IBC-go files second, double-checking if anything is overwritten
        let errors_ibc = WalkDir::new(from_dir_ibc)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let generated_fname = e.file_name().to_owned().into_string().unwrap();
                let prefix = &generated_fname[0..6];

                let target_fname = format!(
                    "{}/{}",
                    to_dir.display(),
                    generated_fname,
                );

                // If it's a cosmos-relevant file and it exists, we should not overwrite it.
                if Path::new(&target_fname).exists() && prefix.eq("cosmos") {
                    let original_cosmos_file = std::fs::read(target_fname.clone()).unwrap();
                    let new_cosmos_file = std::fs::read(e.path()).unwrap();
                    if original_cosmos_file != new_cosmos_file {
                        println!(
                            "[warn ] Cosmos-related file exists already {}! Ignoring the one generated from IBC-go {:?}",
                            target_fname, e.path()
                        );
                    }
                    Ok(0)
                } else {
                    copy(
                        e.path(),
                        target_fname,
                    )
                }
            })
            .filter_map(|e| e.err())
            .collect::<Vec<_>>();

        if !errors_ibc.is_empty() {
            for e in errors_ibc {
                println!("[error] Error while copying IBC-go compiled file: {}", e);
            }

            panic!("[error] Aborted.");
        }
    }
}
