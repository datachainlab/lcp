use crate::{prelude::*, HostClientReader};
use alloc::{collections::btree_map::BTreeMap, rc::Rc};
use commitments::{
    MisbehaviourProxyMessage, PrevState, StateID, UpdateStateProxyMessage, ValidationContext,
    VerifyMembershipProxyMessage,
};
use core::cell::RefCell;
use crypto::Keccak256;
use ibc_core_client::context::{
    consensus_state::ConsensusState,
    prelude::{ClientStateCommon, ClientStateExecution, ClientStateValidation},
    types::error::ClientError,
    ClientExecutionContext, ClientValidationContext,
};
use ibc_core_client::types::{Height as IBCHeight, Status};
use ibc_core_commitment_types::commitment::{CommitmentPrefix, CommitmentProofBytes};
use ibc_core_host_types::{
    error::{DecodingError, HostError},
    identifiers::{ClientId, ClientType},
    path::{ClientConsensusStatePath, ClientStatePath, PathBytes},
};
use ibc_primitives::{proto::Any as IBCAny, Timestamp};
use lcp_types::Height;

pub struct IBCContext<'a, CliS, ConS>
where
    ConS: ConsensusState,
{
    pub parent: &'a dyn HostClientReader,
    pub prev_client_states: Rc<RefCell<BTreeMap<ClientStatePath, CliS>>>,
    pub prev_consensus_states: Rc<RefCell<BTreeMap<ClientConsensusStatePath, ConS>>>,
    pub post_client_states: BTreeMap<ClientStatePath, CliS>,
    pub post_consensus_states: BTreeMap<ClientConsensusStatePath, ConS>,
    _marker: core::marker::PhantomData<(CliS, ConS)>,
}

impl<'a, CliS, ConS> IBCContext<'a, CliS, ConS>
where
    CliS: Clone + TryFrom<IBCAny, Error = DecodingError>,
    ConS: Clone + TryFrom<IBCAny, Error = DecodingError> + ConsensusState,
{
    pub fn new(parent: &'a dyn HostClientReader) -> Self {
        Self {
            parent,
            prev_client_states: Rc::new(RefCell::new(BTreeMap::new())),
            prev_consensus_states: Rc::new(RefCell::new(BTreeMap::new())),
            post_client_states: BTreeMap::new(),
            post_consensus_states: BTreeMap::new(),
            _marker: core::marker::PhantomData,
        }
    }

    pub fn client_state(&self, client_id: &ClientId) -> Result<CliS, HostError> {
        // check if the client state is already stored
        if let Some(client_state) = self
            .post_client_states
            .get(&ClientStatePath(client_id.clone()))
        {
            return Ok(client_state.clone());
        }

        let any_client_state: IBCAny = self
            .parent
            .client_state(&client_id.clone().into())
            .map_err(|e| HostError::failed_to_retrieve(e.to_string()))?
            .into();
        let client_state =
            CliS::try_from(any_client_state).map_err(|e| HostError::InvalidState {
                description: e.to_string(),
            })?;
        self.prev_client_states
            .borrow_mut()
            .insert(client_id.clone().into(), client_state.clone());
        Ok(client_state)
    }

    pub fn consensus_state(
        &self,
        client_cons_state_path: &ClientConsensusStatePath,
    ) -> Result<ConS, HostError> {
        // check if the consensus state is already stored
        if let Some(consensus_state) = self.post_consensus_states.get(client_cons_state_path) {
            return Ok(consensus_state.clone());
        }

        let height = Height::new(
            client_cons_state_path.revision_number,
            client_cons_state_path.revision_height,
        );
        match self
            .parent
            .consensus_state(&client_cons_state_path.client_id.clone().into(), &height)
        {
            Ok(any_consensus_state) => {
                let any_consensus_state = IBCAny::from(any_consensus_state);
                let consensus_state = ConS::try_from(any_consensus_state).unwrap();
                self.prev_consensus_states
                    .borrow_mut()
                    .insert(client_cons_state_path.clone(), consensus_state.clone());
                Ok(consensus_state)
            }
            Err(e) => Err(HostError::failed_to_retrieve(e.to_string())),
        }
    }

    pub fn gen_initialize_state_proxy_message<F>(
        &self,
        gen_state_id: F,
    ) -> ((CliS, ConS), UpdateStateProxyMessage)
    where
        F: Fn(&CliS, &ConS) -> StateID,
    {
        assert_eq!(self.post_client_states.len(), 1);
        assert_eq!(self.post_consensus_states.len(), 1);

        let (_, post_client_state) = self.post_client_states.iter().next().unwrap();
        let (post_consensus_state_path, post_consensus_state) =
            self.post_consensus_states.iter().next().unwrap();

        let post_height = Height::new(
            post_consensus_state_path.revision_number,
            post_consensus_state_path.revision_height,
        );
        let post_state_id = gen_state_id(post_client_state, post_consensus_state);

        (
            (post_client_state.clone(), post_consensus_state.clone()),
            UpdateStateProxyMessage {
                prev_height: None,
                prev_state_id: None,
                post_height,
                post_state_id,
                timestamp: post_consensus_state
                    .timestamp()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                context: ValidationContext::Empty,
                emitted_states: Default::default(),
            },
        )
    }

    pub fn gen_update_state_proxy_message<F>(
        &self,
        client_id: &ClientId,
        gen_state_id: F,
    ) -> ((CliS, ConS), UpdateStateProxyMessage)
    where
        F: Fn(&CliS, &ConS) -> StateID,
    {
        assert_eq!(self.prev_client_states.borrow().len(), 1);
        assert_eq!(self.prev_consensus_states.borrow().len(), 1);
        assert_eq!(self.post_client_states.len(), 1);
        assert_eq!(self.post_consensus_states.len(), 1);

        let prev_client_states = self.prev_client_states.borrow();
        let (prev_client_state_path, prev_client_state) = prev_client_states.iter().next().unwrap();
        assert_eq!(&prev_client_state_path.0, client_id);
        let prev_consensus_states = self.prev_consensus_states.borrow();
        let (prev_consensus_state_path, prev_consensus_state) =
            prev_consensus_states.iter().next().unwrap();
        assert_eq!(&prev_consensus_state_path.client_id, client_id);

        let prev_height = Height::new(
            prev_consensus_state_path.revision_number,
            prev_consensus_state_path.revision_height,
        );
        let prev_state_id = gen_state_id(prev_client_state, prev_consensus_state);

        let (post_client_state_path, post_client_state) =
            self.post_client_states.iter().next().unwrap();
        assert_eq!(&post_client_state_path.0, client_id);
        let (post_consensus_state_path, post_consensus_state) =
            self.post_consensus_states.iter().next().unwrap();
        assert_eq!(&post_consensus_state_path.client_id, client_id);

        let post_height = Height::new(
            post_consensus_state_path.revision_number,
            post_consensus_state_path.revision_height,
        );
        let post_state_id = gen_state_id(post_client_state, post_consensus_state);

        (
            (post_client_state.clone(), post_consensus_state.clone()),
            UpdateStateProxyMessage {
                prev_height: Some(prev_height),
                prev_state_id: Some(prev_state_id),
                post_height,
                post_state_id,
                timestamp: post_consensus_state
                    .timestamp()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                context: ValidationContext::Empty,
                emitted_states: Default::default(),
            },
        )
    }

    pub fn gen_misbehaviour_proxy_message<F>(
        &self,
        _client_id: &ClientId,
        client_message: IBCAny,
        gen_state_id: F,
    ) -> MisbehaviourProxyMessage
    where
        F: Fn(&CliS, &ConS) -> StateID,
    {
        assert!(self.prev_client_states.borrow().len() == 1);
        assert!(self.prev_consensus_states.borrow().len() > 0);

        let prev_client_states = self.prev_client_states.borrow();
        let prev_consensus_states = self.prev_consensus_states.borrow();

        let (_, prev_client_state) = prev_client_states.iter().next().unwrap();

        let prev_states = prev_consensus_states
            .iter()
            .map(|(path, state)| {
                let height = Height::new(path.revision_number, path.revision_height);
                PrevState {
                    height,
                    state_id: gen_state_id(prev_client_state, state),
                }
            })
            .collect();

        MisbehaviourProxyMessage {
            prev_states,
            context: ValidationContext::Empty,
            client_message: client_message.into(),
        }
    }

    pub fn gen_membership_proxy_message<F>(
        &self,
        client_id: &ClientId,
        prefix: CommitmentPrefix,
        path: String,
        value: Option<Vec<u8>>,
        gen_state_id: F,
    ) -> VerifyMembershipProxyMessage
    where
        F: Fn(&CliS, &ConS) -> StateID,
    {
        assert_eq!(self.prev_consensus_states.borrow().len(), 1);
        let client_state = self.client_state(client_id).unwrap();
        let prev_consensus_states = self.prev_consensus_states.borrow();
        let (cons_path, consensus_state) = prev_consensus_states.iter().next().unwrap();
        let state_id = gen_state_id(&client_state, consensus_state);
        VerifyMembershipProxyMessage::new(
            prefix.into_vec(),
            path,
            value.map(|v| v.keccak256()),
            Height::new(cons_path.revision_number, cons_path.revision_height),
            state_id,
        )
    }
}

pub struct IBCHandler;

pub struct CreateExecutionResult(pub IBCHeight);

pub enum UpdateExecutionResult {
    Success(Vec<IBCHeight>),
    Misbehaviour,
}

impl IBCHandler {
    pub fn create_client<Ctx: ClientExecutionContext>(
        hctx: &dyn HostClientReader,
        ctx: &mut Ctx,
        timestamp: Timestamp,
        client_state: IBCAny,
        consensus_state: IBCAny,
    ) -> Result<CreateExecutionResult, ClientError> {
        let client_id = match Ctx::ClientStateMut::try_from(client_state.clone()) {
            Ok(client_state) => Self::generate_dummy_client_id(hctx, client_state.client_type())?,
            Err(_) => {
                return Err(ClientError::ClientSpecific {
                    description: "failed to decode client state".to_string(),
                })
            }
        };
        Self::validate_create_client(
            ctx,
            timestamp,
            &client_id,
            client_state.clone(),
            consensus_state.clone(),
        )?;
        Self::execute_create_client(
            ctx,
            &client_id,
            client_state.clone(),
            consensus_state.clone(),
        )
    }

    pub fn validate_create_client<Ctx: ClientValidationContext>(
        ctx: &Ctx,
        timestamp: Timestamp,
        client_id: &ClientId,
        client_state: IBCAny,
        consensus_state: IBCAny,
    ) -> Result<(), ClientError> {
        let client_state = match Ctx::ClientStateRef::try_from(client_state) {
            Ok(client_state) => client_state,
            Err(_) => {
                return Err(ClientError::ClientSpecific {
                    description: "failed to decode client state".to_string(),
                })
            }
        };
        let status = client_state.status(ctx, client_id)?;
        if status.is_frozen() {
            return Err(ClientError::InvalidStatus(Status::Frozen));
        }
        client_state.verify_consensus_state(consensus_state, &timestamp)?;
        Ok(())
    }

    pub fn execute_create_client<Ctx: ClientExecutionContext>(
        ctx: &mut Ctx,
        client_id: &ClientId,
        client_state: IBCAny,
        consensus_state: IBCAny,
    ) -> Result<CreateExecutionResult, ClientError> {
        let client_state = match Ctx::ClientStateMut::try_from(client_state) {
            Ok(client_state) => client_state,
            Err(_) => {
                return Err(ClientError::ClientSpecific {
                    description: "failed to decode client state".to_string(),
                })
            }
        };
        client_state.initialise(ctx, client_id, consensus_state)?;
        let latest_height = client_state.latest_height();
        Ok(CreateExecutionResult(latest_height))
    }

    pub fn update_client<Ctx: ClientExecutionContext>(
        ctx: &mut Ctx,
        client_id: &ClientId,
        client_message: IBCAny,
    ) -> Result<UpdateExecutionResult, ClientError> {
        Self::validate_update_client(ctx, client_id, client_message.clone())?;
        Self::execute_update_client(ctx, client_id, client_message)
    }

    pub fn validate_update_client<Ctx: ClientValidationContext>(
        ctx: &Ctx,
        client_id: &ClientId,
        client_message: IBCAny,
    ) -> Result<(), ClientError> {
        let client_state = ctx.client_state(client_id)?;
        client_state.status(ctx, client_id)?.verify_is_active()?;
        client_state.verify_client_message(ctx, client_id, client_message)?;
        Ok(())
    }

    pub fn execute_update_client<Ctx: ClientExecutionContext>(
        ctx: &mut Ctx,
        client_id: &ClientId,
        client_message: IBCAny,
    ) -> Result<UpdateExecutionResult, ClientError> {
        let client_state = ctx.client_state(client_id)?;
        let found_misbehaviour =
            client_state.check_for_misbehaviour(ctx, client_id, client_message.clone())?;
        if found_misbehaviour {
            client_state.update_state_on_misbehaviour(ctx, client_id, client_message)?;
            Ok(UpdateExecutionResult::Misbehaviour)
        } else {
            let consensus_heights =
                client_state.update_state(ctx, client_id, client_message.clone())?;
            if consensus_heights.len() != 1 {
                // return Err(ClientError::InvalidRawClientState {
                //     client_id: client_id.clone(),
                //     reason: "expected exactly one consensus height".to_string(),
                // });
                panic!("expected exactly one consensus height");
            }
            Ok(UpdateExecutionResult::Success(consensus_heights))
        }
    }

    pub fn verify_membership<Ctx: ClientValidationContext>(
        ctx: &Ctx,
        client_id: &ClientId,
        prefix: &CommitmentPrefix,
        proof_height: IBCHeight,
        proof: &CommitmentProofBytes,
        path: PathBytes,
        value: Vec<u8>,
    ) -> Result<(), ClientError> {
        let client_state = ctx.client_state(client_id)?;
        client_state.status(ctx, client_id)?.verify_is_active()?;
        client_state.validate_proof_height(proof_height)?;
        let consensus_state = ctx.consensus_state(&ClientConsensusStatePath::new(
            client_id.clone(),
            proof_height.revision_number(),
            proof_height.revision_height(),
        ))?;
        client_state.verify_membership_raw(prefix, proof, consensus_state.root(), path, value)
    }

    pub fn verify_non_membership<Ctx: ClientValidationContext>(
        ctx: &Ctx,
        client_id: &ClientId,
        prefix: &CommitmentPrefix,
        proof_height: IBCHeight,
        proof: &CommitmentProofBytes,
        path: PathBytes,
    ) -> Result<(), ClientError> {
        let client_state = ctx.client_state(client_id)?;
        client_state.status(ctx, client_id)?.verify_is_active()?;
        client_state.validate_proof_height(proof_height)?;
        let consensus_state = ctx.consensus_state(&ClientConsensusStatePath::new(
            client_id.clone(),
            proof_height.revision_number(),
            proof_height.revision_height(),
        ))?;
        client_state.verify_non_membership_raw(prefix, proof, consensus_state.root(), path)
    }

    fn generate_dummy_client_id(
        hctx: &dyn HostClientReader,
        client_type: ClientType,
    ) -> Result<ClientId, ClientError> {
        loop {
            let counter = 0;
            let client_id = client_type.build_client_id(counter);
            if !hctx.client_exists(&client_id.clone().into()) {
                return Ok(client_id);
            }
        }
    }
}

pub mod exports {
    pub use ibc_core_client::context::{
        ClientExecutionContext, ClientValidationContext, ExtClientValidationContext,
    };
    pub use ibc_core_client::types::Height;
    pub use ibc_core_host_types::error::HostError;
    pub use ibc_core_host_types::identifiers::ClientId;
    pub use ibc_core_host_types::path::{ClientConsensusStatePath, ClientStatePath};
    pub use ibc_primitives::Timestamp;
}

#[macro_export]
macro_rules! impl_ibc_context {
    ($name:ident, $client_state:ty, $consensus_state:ty) => {
        pub struct $name<'a>(pub $crate::ibc::IBCContext<'a, $client_state, $consensus_state>);

        impl<'a> $name<'a> {
            pub fn new(parent: &'a dyn $crate::HostClientReader) -> Self {
                Self($crate::ibc::IBCContext::new(parent))
            }
        }

        impl<'a> core::ops::Deref for $name<'a> {
            type Target = $crate::ibc::IBCContext<'a, $client_state, $consensus_state>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<'a> $crate::ibc::exports::ClientValidationContext for $name<'a> {
            type ClientStateRef = $client_state;
            type ConsensusStateRef = $consensus_state;

            fn client_state(
                &self,
                client_id: &$crate::ibc::exports::ClientId,
            ) -> Result<Self::ClientStateRef, $crate::ibc::exports::HostError> {
                $crate::ibc::IBCContext::client_state(self, client_id)
            }

            fn consensus_state(
                &self,
                client_cons_state_path: &$crate::ibc::exports::ClientConsensusStatePath,
            ) -> Result<Self::ConsensusStateRef, $crate::ibc::exports::HostError> {
                $crate::ibc::IBCContext::consensus_state(self, client_cons_state_path)
            }

            fn client_update_meta(
                &self,
                client_id: &$crate::ibc::exports::ClientId,
                height: &$crate::ibc::exports::Height,
            ) -> Result<
                (
                    $crate::ibc::exports::Timestamp,
                    $crate::ibc::exports::Height,
                ),
                $crate::ibc::exports::HostError,
            > {
                Err($crate::ibc::exports::HostError::Other {
                    description: alloc::format!(
                        "client_update_meta not implemented for client_id: {}, height: {}",
                        client_id,
                        height
                    ),
                })
            }
        }

        impl<'a> $crate::ibc::exports::ClientExecutionContext for $name<'a> {
            type ClientStateMut = $client_state;

            fn store_client_state(
                &mut self,
                client_state_path: $crate::ibc::exports::ClientStatePath,
                client_state: Self::ClientStateRef,
            ) -> Result<(), $crate::ibc::exports::HostError> {
                self.0
                    .post_client_states
                    .insert(client_state_path, client_state);
                Ok(())
            }

            fn store_consensus_state(
                &mut self,
                consensus_state_path: $crate::ibc::exports::ClientConsensusStatePath,
                consensus_state: Self::ConsensusStateRef,
            ) -> Result<(), $crate::ibc::exports::HostError> {
                self.0
                    .post_consensus_states
                    .insert(consensus_state_path, consensus_state);
                Ok(())
            }

            fn delete_consensus_state(
                &mut self,
                _consensus_state_path: $crate::ibc::exports::ClientConsensusStatePath,
            ) -> Result<(), $crate::ibc::exports::HostError> {
                Ok(())
            }

            fn store_update_meta(
                &mut self,
                _client_id: $crate::ibc::exports::ClientId,
                _height: $crate::ibc::exports::Height,
                _host_timestamp: $crate::ibc::exports::Timestamp,
                _host_height: $crate::ibc::exports::Height,
            ) -> Result<(), $crate::ibc::exports::HostError> {
                Ok(())
            }

            fn delete_update_meta(
                &mut self,
                _client_id: $crate::ibc::exports::ClientId,
                _height: $crate::ibc::exports::Height,
            ) -> Result<(), $crate::ibc::exports::HostError> {
                Ok(())
            }
        }
    };
}

#[macro_export]
macro_rules! impl_ibc_context_ext {
    ($name:ident, $client_state:ty, $consensus_state:ty) => {
        $crate::impl_ibc_context!($name, $client_state, $consensus_state);

        impl<'a> $crate::ibc::exports::ExtClientValidationContext for $name<'a> {
            fn host_timestamp(
                &self,
            ) -> Result<$crate::ibc::exports::Timestamp, $crate::ibc::exports::HostError> {
                Ok(self.parent.host_timestamp().into())
            }

            fn host_height(
                &self,
            ) -> Result<$crate::ibc::exports::Height, $crate::ibc::exports::HostError> {
                // NOTE: This is a dummy height, as the ELCs do not have a concept of height.
                Ok($crate::ibc::exports::Height::min(0))
            }

            fn consensus_state_heights(
                &self,
                client_id: &$crate::ibc::exports::ClientId,
            ) -> Result<Vec<$crate::ibc::exports::Height>, $crate::ibc::exports::HostError> {
                Ok(Vec::new())
            }

            fn next_consensus_state(
                &self,
                client_id: &$crate::ibc::exports::ClientId,
                height: &$crate::ibc::exports::Height,
            ) -> Result<Option<Self::ConsensusStateRef>, $crate::ibc::exports::HostError> {
                Ok(None)
            }

            fn prev_consensus_state(
                &self,
                client_id: &$crate::ibc::exports::ClientId,
                height: &$crate::ibc::exports::Height,
            ) -> Result<Option<Self::ConsensusStateRef>, $crate::ibc::exports::HostError> {
                Ok(None)
            }
        }
    };
}
