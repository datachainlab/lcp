use core::ops::Deref;

use crate::errors::Error;
use ibc::clients::ics07_tendermint::header::{
    Header as TendermintHeader, TENDERMINT_HEADER_TYPE_URL,
};
use lcp_proto::google::protobuf::Any as IBCAny;
use light_client::types::Any;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Header(pub(crate) TendermintHeader);

impl Deref for Header {
    type Target = TendermintHeader;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Header> for TendermintHeader {
    fn from(value: Header) -> Self {
        value.0
    }
}

impl From<TendermintHeader> for Header {
    fn from(value: TendermintHeader) -> Self {
        Self(value)
    }
}

impl From<Header> for Any {
    fn from(value: Header) -> Self {
        IBCAny::from(value.0).into()
    }
}

impl TryFrom<Any> for Header {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: IBCAny = value.into();
        if any.type_url == TENDERMINT_HEADER_TYPE_URL {
            Ok(Self(TendermintHeader::try_from(any).map_err(Error::ics02)?))
        } else {
            Err(Error::unexpected_client_type(any.type_url))
        }
    }
}
