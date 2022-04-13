use ibc::{
    core::{ics02_client::error::Error as ICS02Error, ics24_host::identifier::ClientId},
    timestamp::Timestamp,
    Height,
};
use prost_types::Any;
use std::string::String;

pub trait LightClientReader {
    fn client_type(&self, client_id: &ClientId) -> Result<String, ICS02Error>;
    fn client_state(&self, client_id: &ClientId) -> Result<Any, ICS02Error>;
    fn consensus_state(&self, client_id: &ClientId, height: Height) -> Result<Any, ICS02Error>;
    fn host_height(&self) -> Height;
    fn host_timestamp(&self) -> Timestamp;
}

pub trait LightClientKeeper {
    /// Called upon successful client creation
    fn store_client_type(
        &mut self,
        client_id: ClientId,
        client_type: String,
    ) -> Result<(), ICS02Error>;

    /// Called upon successful client creation and update
    fn store_any_client_state(
        &mut self,
        client_id: ClientId,
        client_state: Any,
    ) -> Result<(), ICS02Error>;

    /// Called upon successful client creation and update
    fn store_any_consensus_state(
        &mut self,
        client_id: ClientId,
        height: Height,
        consensus_state: Any,
    ) -> Result<(), ICS02Error>;

    /// Called upon client creation.
    /// Increases the counter which keeps track of how many clients have been created.
    /// Should never fail.
    fn increase_client_counter(&mut self);

    /// Called upon successful client update.
    /// Implementations are expected to use this to record the specified time as the time at which
    /// this update (or header) was processed.
    fn store_update_time(
        &mut self,
        client_id: ClientId,
        height: Height,
        timestamp: Timestamp,
    ) -> Result<(), ICS02Error>;

    /// Called upon successful client update.
    /// Implementations are expected to use this to record the specified height as the height at
    /// at which this update (or header) was processed.
    fn store_update_height(
        &mut self,
        client_id: ClientId,
        height: Height,
        host_height: Height,
    ) -> Result<(), ICS02Error>;
}
