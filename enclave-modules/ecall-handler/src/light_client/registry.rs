use crate::light_client::Error;
use crate::prelude::*;
use context::Context;
use crypto::Signer;
use lcp_types::ClientId;
use light_client::{ClientReader, LightClient, LightClientResolver, RegistryError};
use store::KVStore;

pub fn get_light_client_by_client_id<'a, R: LightClientResolver, S: KVStore, K: Signer>(
    ctx: &'a Context<R, S, K>,
    client_id: &ClientId,
) -> Result<&'a Box<dyn LightClient>, Error> {
    let any_client_state = ctx.client_state(client_id)?.to_proto();
    ctx.get_light_client(any_client_state.type_url.as_ref())
        .ok_or(Error::light_client_registry(
            RegistryError::type_url_not_found(any_client_state.type_url),
        ))
}
