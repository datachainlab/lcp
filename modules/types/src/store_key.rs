use crate::{prelude::*, Height};

pub fn client_type(client_id: &str) -> String {
    format!("clients/{client_id}/clientType")
}

pub fn client_state(client_id: &str) -> String {
    format!("clients/{client_id}/clientState")
}

pub fn client_state_at_height(client_id: &str, height: &Height) -> String {
    format!(
        "clients/{}/clientStates/{}-{}",
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

pub fn client_state_at_height_bytes(client_id: &str, height: &Height) -> Vec<u8> {
    client_state_at_height(client_id, height).into_bytes()
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
    }
}
