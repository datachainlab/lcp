use crate::errors::Error;
use core::ops::Deref;
use ibc::mock::header::{MockHeader, MOCK_HEADER_TYPE_URL};
use ibc::mock::misbehaviour::{Misbehaviour as MockMisbehaviour, MOCK_MISBEHAVIOUR_TYPE_URL};
use light_client::types::proto::google::protobuf::Any as ProtoAny;
use light_client::types::Any;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ClientMessage {
    Header(Header),
    Misbehaviour(Misbehaviour),
}

impl TryFrom<Any> for ClientMessage {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        if value.type_url == MOCK_HEADER_TYPE_URL {
            Ok(Self::Header(Header::try_from(value)?))
        } else if value.type_url == MOCK_MISBEHAVIOUR_TYPE_URL {
            Ok(Self::Misbehaviour(Misbehaviour::try_from(value)?))
        } else {
            Err(Error::unexpected_client_type(value.type_url.clone()))
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Header(pub(crate) MockHeader);

impl Deref for Header {
    type Target = MockHeader;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MockHeader> for Header {
    fn from(value: MockHeader) -> Self {
        Self(value)
    }
}

impl TryFrom<Any> for Header {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: ProtoAny = value.into();
        if any.type_url == MOCK_HEADER_TYPE_URL {
            Ok(Self(MockHeader::try_from(any).map_err(Error::ics02)?))
        } else {
            Err(Error::unexpected_client_type(any.type_url))
        }
    }
}

impl From<Header> for Any {
    fn from(value: Header) -> Self {
        ProtoAny::from(value.0).into()
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Misbehaviour(pub(crate) MockMisbehaviour);

impl Deref for Misbehaviour {
    type Target = MockMisbehaviour;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<MockMisbehaviour> for Misbehaviour {
    fn from(value: MockMisbehaviour) -> Self {
        Self(value)
    }
}

impl TryFrom<Any> for Misbehaviour {
    type Error = Error;

    fn try_from(value: Any) -> Result<Self, Self::Error> {
        let any: ProtoAny = value.into();
        if any.type_url == MOCK_MISBEHAVIOUR_TYPE_URL {
            Ok(Self(MockMisbehaviour::try_from(any).map_err(Error::ics02)?))
        } else {
            Err(Error::unexpected_client_type(any.type_url))
        }
    }
}

impl From<Misbehaviour> for Any {
    fn from(value: Misbehaviour) -> Self {
        ProtoAny::from(value.0).into()
    }
}
