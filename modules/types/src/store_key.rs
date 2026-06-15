use crate::{prelude::*, Height};

pub fn client_type(client_id: &str) -> String {
    format!("clients/{client_id}/clientType")
}

pub fn client_state(client_id: &str) -> String {
    format!("clients/{client_id}/clientState")
}

/// Per-height client_state key (per-height client_state design).
///
/// The singleton `client_state(client_id)` continues to be written and kept
/// as "the latest" client_state for compatibility with every `ctx.client_state`
/// reader in the ELC layer. In parallel, each speculative-batch commit also
/// writes the client_state at its specific height under this key, so that
/// `verify_expected_base_state_in_tx` and the new `query_client_at_height`
/// RPC can resolve past anchors and let drift recovery start a new stream
/// from any committed past entry instead of requiring the operator to run
/// the serial-heal procedure documented in the legacy serial-heal procedure.
pub fn client_state_at_height(client_id: &str, height: &Height) -> String {
    format!(
        "clients/{}/clientStates/{}-{}",
        client_id,
        height.revision_number(),
        height.revision_height()
    )
}

pub fn state_id(client_id: &str, height: &Height) -> String {
    format!(
        "clients/{}/stateIds/{}-{}",
        client_id,
        height.revision_number(),
        height.revision_height()
    )
}

pub fn consensus_state(client_id: &str, height: &Height) -> String {
    format!(
        "clients/{}/consensusStates/{}-{}",
        client_id,
        height.revision_number(),
        height.revision_height()
    )
}

pub fn client_type_bytes(client_id: &str) -> Vec<u8> {
    client_type(client_id).into_bytes()
}

pub fn client_state_bytes(client_id: &str) -> Vec<u8> {
    client_state(client_id).into_bytes()
}

/// Per-height client_state key bytes. See [`client_state_at_height`].
pub fn client_state_at_height_bytes(client_id: &str, height: &Height) -> Vec<u8> {
    client_state_at_height(client_id, height).into_bytes()
}

pub fn state_id_bytes(client_id: &str, height: &Height) -> Vec<u8> {
    state_id(client_id, height).into_bytes()
}

pub fn consensus_state_bytes(client_id: &str, height: &Height) -> Vec<u8> {
    consensus_state(client_id, height).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_client_store_keys() {
        let height = Height::new(1, 23);
        assert_eq!(
            client_type("07-tendermint-0"),
            "clients/07-tendermint-0/clientType"
        );
        assert_eq!(
            client_state("07-tendermint-0"),
            "clients/07-tendermint-0/clientState"
        );
        assert_eq!(
            client_state_at_height("07-tendermint-0", &height),
            "clients/07-tendermint-0/clientStates/1-23"
        );
        assert_eq!(
            consensus_state("07-tendermint-0", &height),
            "clients/07-tendermint-0/consensusStates/1-23"
        );
        assert_eq!(
            state_id("07-tendermint-0", &height),
            "clients/07-tendermint-0/stateIds/1-23"
        );
    }

    #[test]
    fn per_height_client_state_key_is_distinct_from_singleton() {
        // Plural form prevents the per-height key from accidentally aliasing
        // the singleton "clientState" path even when serialized as bytes.
        let height = Height::new(0, 0);
        let singleton = client_state("c");
        let at_zero = client_state_at_height("c", &height);
        assert_ne!(singleton.as_bytes(), at_zero.as_bytes());
        assert!(at_zero.starts_with(&format!("{singleton}s/")));
    }
}
