use crate::light_client::LightClientHandlerError as Error;
use context::Context;
use ibc::core::ics24_host::identifier::ClientId;
use light_client::{ClientReader, LightClient, LightClientError, LightClientSource};
use std::boxed::Box;
use store::KVStore;

pub fn get_light_client_by_client_id<'l, S: KVStore, L: LightClientSource<'l>>(
    ctx: &Context<S>,
    client_id: &ClientId,
) -> Result<&'l Box<dyn LightClient>, Error> {
    let any_client_state = ctx
        .client_state(client_id)
        .map_err(Error::ICS02Error)?
        .to_proto();
    L::get_light_client(any_client_state.type_url.as_ref()).ok_or(Error::LightClientError(
        LightClientError::TypeUrlNotFoundError(any_client_state.type_url),
    ))
}
