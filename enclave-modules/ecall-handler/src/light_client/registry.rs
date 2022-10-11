use crate::light_client::Error;
use crate::prelude::*;
use context::Context;
use ibc::core::ics24_host::identifier::ClientId;
use light_client::{ClientReader, LightClient};
use light_client_registry::{Error as LightClientRegistryError, LightClientSource};
use store::KVStore;

pub fn get_light_client_by_client_id<'l, S: KVStore, L: LightClientSource<'l>>(
    ctx: &Context<S>,
    client_id: &ClientId,
) -> Result<&'l Box<dyn LightClient>, Error> {
    let any_client_state = ctx
        .client_state(client_id)
        .map_err(Error::ics02)?
        .to_proto();
    L::get_light_client(any_client_state.type_url.as_ref()).ok_or(Error::light_client_registry(
        LightClientRegistryError::type_url_not_found(any_client_state.type_url),
    ))
}
