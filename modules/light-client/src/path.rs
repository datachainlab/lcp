use crate::types::{ClientId, Height};
use derive_more::Display;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display("clients/{_0}/clientType")]
pub struct ClientTypePath(pub ClientId);

impl ClientTypePath {
    pub fn new(client_id: &ClientId) -> ClientTypePath {
        ClientTypePath(client_id.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display("clients/{_0}/clientState")]
pub struct ClientStatePath(pub ClientId);

impl ClientStatePath {
    pub fn new(client_id: &ClientId) -> ClientStatePath {
        ClientStatePath(client_id.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
#[display("clients/{client_id}/consensusStates/{epoch}-{height}")]
pub struct ClientConsensusStatePath {
    pub client_id: ClientId,
    pub epoch: u64,
    pub height: u64,
}

impl ClientConsensusStatePath {
    pub fn new(client_id: &ClientId, height: &Height) -> ClientConsensusStatePath {
        ClientConsensusStatePath {
            client_id: client_id.clone(),
            epoch: height.revision_number(),
            height: height.revision_height(),
        }
    }
}
