use crate::errors::Error;
use core::ops::Deref;
use ibc::clients::ics07_tendermint::header::{
    Header as TendermintHeader, TENDERMINT_HEADER_TYPE_URL,
};
use ibc::clients::ics07_tendermint::misbehaviour::{
    Misbehaviour as TendermintMisbehaviour, TENDERMINT_MISBEHAVIOUR_TYPE_URL,
};
use lcp_proto::google::protobuf::Any as ProtoAny;
use light_client::types::Any;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ClientMessage {
    Header(Header),
    Misbehaviour(Misbehaviour),
}

impl From<ClientMessage> for Any {
    fn from(value: ClientMessage) -> Self {
        match value {
            ClientMessage::Header(header) => header.into(),
            ClientMessage::Misbehaviour(misbehaviour) => misbehaviour.into(),
        }
    }
}

impl TryFrom<Any> for ClientMessage {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        if value.type_url == TENDERMINT_HEADER_TYPE_URL {
            Ok(Self::Header(Header::try_from(value)?))
        } else if value.type_url == TENDERMINT_MISBEHAVIOUR_TYPE_URL {
            Ok(Self::Misbehaviour(Misbehaviour::try_from(value)?))
        } else {
            Err(Error::unexpected_client_type(value.type_url.clone()))
        }
    }
}

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
        ProtoAny::from(value.0).into()
    }
}

impl TryFrom<Any> for Header {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: ProtoAny = value.into();
        if any.type_url == TENDERMINT_HEADER_TYPE_URL {
            Ok(Self(TendermintHeader::try_from(any).map_err(Error::ics02)?))
        } else {
            Err(Error::unexpected_client_type(any.type_url))
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Misbehaviour(pub(crate) TendermintMisbehaviour);

impl Deref for Misbehaviour {
    type Target = TendermintMisbehaviour;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Misbehaviour> for TendermintMisbehaviour {
    fn from(value: Misbehaviour) -> Self {
        value.0
    }
}

impl From<TendermintMisbehaviour> for Misbehaviour {
    fn from(value: TendermintMisbehaviour) -> Self {
        Self(value)
    }
}

impl From<Misbehaviour> for Any {
    fn from(value: Misbehaviour) -> Self {
        ProtoAny::from(value.0).into()
    }
}

impl TryFrom<Any> for Misbehaviour {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: ProtoAny = value.into();
        if any.type_url == TENDERMINT_MISBEHAVIOUR_TYPE_URL {
            Ok(Self(
                TendermintMisbehaviour::try_from(any).map_err(Error::ics02)?,
            ))
        } else {
            Err(Error::unexpected_client_type(any.type_url))
        }
    }
}
