#![no_std]
extern crate alloc;
use enclave_runtime::{setup_runtime, Environment, MapLightClientRegistry};

setup_runtime!({
    // simple_logger::SimpleLogger::new()
    //     .with_level(log::LevelFilter::Info)
    //     .init()
    //     .unwrap();
    Environment::new(build_lc_registry())
});

fn build_lc_registry() -> MapLightClientRegistry {
    let mut registry = MapLightClientRegistry::new();
    tendermint_lc::register_implementations(&mut registry);
    registry
}
