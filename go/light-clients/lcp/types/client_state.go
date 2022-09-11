package types

import (
	"bytes"
	"fmt"
	"time"

	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	connectiontypes "github.com/cosmos/ibc-go/v4/modules/core/03-connection/types"
	channeltypes "github.com/cosmos/ibc-go/v4/modules/core/04-channel/types"
	commitmenttypes "github.com/cosmos/ibc-go/v4/modules/core/23-commitment/types"
	host "github.com/cosmos/ibc-go/v4/modules/core/24-host"
	"github.com/cosmos/ibc-go/v4/modules/core/exported"
	"github.com/ethereum/go-ethereum/common"
)

const (
	ModuleName    = "lcp"
	ClientTypeLCP = "lcp-client"
	MrenclaveSize = 32
)

var _ exported.ClientState = (*ClientState)(nil)

func (cs ClientState) ClientType() string {
	return ClientTypeLCP
}

func (cs ClientState) GetLatestHeight() exported.Height {
	return cs.LatestHeight
}

func (cs ClientState) Validate() error {
	if cs.KeyExpiration == 0 {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidClient, "`KeyExpiration` must be non-zero")
	}
	if l := len(cs.Mrenclave); l != MrenclaveSize {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidClient, "`Mrenclave` length must be %v, but got %v", MrenclaveSize, l)
	}
	return nil
}

// Initialization function
// Clients must validate the initial consensus state, and may store any client-specific metadata
// necessary for correct light client operation
func (cs ClientState) Initialize(_ sdk.Context, _ codec.BinaryCodec, _ sdk.KVStore, consensusState exported.ConsensusState) error {
	if len(cs.Keys) != 0 {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidClient, "`Keys` length must be zero")
	}
	if cs.KeyExpiration == 0 {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidClient, "`KeyExpiration` must be non-zero")
	}
	if !cs.LatestHeight.IsZero() {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidClient, "`LatestHeight` must be zero height")
	}
	if l := len(cs.Mrenclave); l != MrenclaveSize {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidClient, "`Mrenclave` length must be %v, but got %v", MrenclaveSize, l)
	}
	consensusState, ok := consensusState.(*ConsensusState)
	if !ok {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidConsensus, "unexpected consensus state type: expected=%T got=%T", &ConsensusState{}, consensusState)
	}
	return nil
}

// Status function
// Clients must return their status. Only Active clients are allowed to process packets.
func (cs ClientState) Status(ctx sdk.Context, clientStore sdk.KVStore, cdc codec.BinaryCodec) exported.Status {
	return exported.Active
}

// Genesis function
func (cs ClientState) ExportMetadata(_ sdk.KVStore) []exported.GenesisMetadata {
	panic("not implemented") // TODO: Implement
}

func (cs ClientState) CheckMisbehaviourAndUpdateState(_ sdk.Context, _ codec.BinaryCodec, _ sdk.KVStore, _ exported.Misbehaviour) (exported.ClientState, error) {
	panic("not implemented") // TODO: Implement
}

func (cs ClientState) CheckSubstituteAndUpdateState(ctx sdk.Context, cdc codec.BinaryCodec, subjectClientStore sdk.KVStore, substituteClientStore sdk.KVStore, substituteClient exported.ClientState) (exported.ClientState, error) {
	panic("not implemented") // TODO: Implement
}

// Upgrade functions
// NOTE: proof heights are not included as upgrade to a new revision is expected to pass only on the last
// height committed by the current revision. Clients are responsible for ensuring that the planned last
// height of the current revision is somehow encoded in the proof verification process.
// This is to ensure that no premature upgrades occur, since upgrade plans committed to by the counterparty
// may be cancelled or modified before the last planned height.
func (cs ClientState) VerifyUpgradeAndUpdateState(ctx sdk.Context, cdc codec.BinaryCodec, store sdk.KVStore, newClient exported.ClientState, newConsState exported.ConsensusState, proofUpgradeClient []byte, proofUpgradeConsState []byte) (exported.ClientState, exported.ConsensusState, error) {
	panic("not implemented") // TODO: Implement
}

// Utility function that zeroes out any client customizable fields in client state
// Ledger enforced fields are maintained while all custom fields are zero values
// Used to verify upgrades
func (cs ClientState) ZeroCustomFields() exported.ClientState {
	panic("not implemented") // TODO: Implement
}

// State verification functions
func (cs ClientState) VerifyClientState(store sdk.KVStore, cdc codec.BinaryCodec, height exported.Height, prefix exported.Prefix, counterpartyClientIdentifier string, proof []byte, clientState exported.ClientState) error {
	merklePath := commitmenttypes.NewMerklePath(host.FullClientStatePath(counterpartyClientIdentifier))
	merklePath, err := commitmenttypes.ApplyPrefix(prefix, merklePath)
	if err != nil {
		return err
	}
	path, err := cdc.Marshal(&merklePath)
	if err != nil {
		return err
	}
	bz, err := cdc.MarshalInterface(clientState)
	if err != nil {
		return err
	}
	return cs.VerifyMembership(fakeBlockTime(), store, cdc, height, 0, 0, proof, path, bz)
}

func (cs ClientState) VerifyClientConsensusState(store sdk.KVStore, cdc codec.BinaryCodec, height exported.Height, counterpartyClientIdentifier string, consensusHeight exported.Height, prefix exported.Prefix, proof []byte, consensusState exported.ConsensusState) error {
	merklePath := commitmenttypes.NewMerklePath(host.FullConsensusStatePath(counterpartyClientIdentifier, consensusHeight))
	merklePath, err := commitmenttypes.ApplyPrefix(prefix, merklePath)
	if err != nil {
		return err
	}
	path, err := cdc.Marshal(&merklePath)
	if err != nil {
		return err
	}
	bz, err := cdc.MarshalInterface(consensusState)
	if err != nil {
		return err
	}
	return cs.VerifyMembership(fakeBlockTime(), store, cdc, height, 0, 0, proof, path, bz)
}

func (cs ClientState) VerifyConnectionState(store sdk.KVStore, cdc codec.BinaryCodec, height exported.Height, prefix exported.Prefix, proof []byte, connectionID string, counterpartyConnection exported.ConnectionI) error {
	merklePath := commitmenttypes.NewMerklePath(host.ConnectionPath(connectionID))
	merklePath, err := commitmenttypes.ApplyPrefix(prefix, merklePath)
	if err != nil {
		return err
	}
	path, err := cdc.Marshal(&merklePath)
	if err != nil {
		return err
	}
	connectionEnd, ok := counterpartyConnection.(connectiontypes.ConnectionEnd)
	if !ok {
		return sdkerrors.Wrapf(sdkerrors.ErrInvalidType, "invalid connection type %T", counterpartyConnection)
	}
	bz, err := cdc.Marshal(&connectionEnd)
	if err != nil {
		return err
	}
	return cs.VerifyMembership(fakeBlockTime(), store, cdc, height, 0, 0, proof, path, bz)
}

func (cs ClientState) VerifyChannelState(store sdk.KVStore, cdc codec.BinaryCodec, height exported.Height, prefix exported.Prefix, proof []byte, portID string, channelID string, counterpartyChannel exported.ChannelI) error {
	merklePath := commitmenttypes.NewMerklePath(host.ChannelPath(portID, channelID))
	merklePath, err := commitmenttypes.ApplyPrefix(prefix, merklePath)
	if err != nil {
		return err
	}
	path, err := cdc.Marshal(&merklePath)
	if err != nil {
		return err
	}
	channelEnd, ok := counterpartyChannel.(channeltypes.Channel)
	if !ok {
		return sdkerrors.Wrapf(sdkerrors.ErrInvalidType, "invalid channel type %T", counterpartyChannel)
	}
	bz, err := cdc.Marshal(&channelEnd)
	if err != nil {
		return err
	}
	return cs.VerifyMembership(fakeBlockTime(), store, cdc, height, 0, 0, proof, path, bz)
}

func (cs ClientState) VerifyPacketCommitment(ctx sdk.Context, store sdk.KVStore, cdc codec.BinaryCodec, height exported.Height, delayTimePeriod uint64, delayBlockPeriod uint64, prefix exported.Prefix, proof []byte, portID string, channelID string, sequence uint64, commitmentBytes []byte) error {
	merklePath := commitmenttypes.NewMerklePath(host.PacketCommitmentPath(portID, channelID, sequence))
	merklePath, err := commitmenttypes.ApplyPrefix(prefix, merklePath)
	if err != nil {
		return err
	}
	path, err := cdc.Marshal(&merklePath)
	if err != nil {
		return err
	}
	return cs.VerifyMembershipWithContext(ctx, store, cdc, height, delayTimePeriod, delayBlockPeriod, proof, path, commitmentBytes)
}

func (cs ClientState) VerifyPacketAcknowledgement(ctx sdk.Context, store sdk.KVStore, cdc codec.BinaryCodec, height exported.Height, delayTimePeriod uint64, delayBlockPeriod uint64, prefix exported.Prefix, proof []byte, portID string, channelID string, sequence uint64, acknowledgement []byte) error {
	merklePath := commitmenttypes.NewMerklePath(host.PacketAcknowledgementPath(portID, channelID, sequence))
	merklePath, err := commitmenttypes.ApplyPrefix(prefix, merklePath)
	if err != nil {
		return err
	}
	path, err := cdc.Marshal(&merklePath)
	if err != nil {
		return err
	}
	return cs.VerifyMembershipWithContext(ctx, store, cdc, height, delayTimePeriod, delayBlockPeriod, proof, path, channeltypes.CommitAcknowledgement(acknowledgement))
}

func (cs ClientState) VerifyPacketReceiptAbsence(ctx sdk.Context, store sdk.KVStore, cdc codec.BinaryCodec, height exported.Height, delayTimePeriod uint64, delayBlockPeriod uint64, prefix exported.Prefix, proof []byte, portID string, channelID string, sequence uint64) error {
	merklePath := commitmenttypes.NewMerklePath(host.PacketReceiptPath(portID, channelID, sequence))
	merklePath, err := commitmenttypes.ApplyPrefix(prefix, merklePath)
	if err != nil {
		return err
	}
	path, err := cdc.Marshal(&merklePath)
	if err != nil {
		return err
	}
	return cs.VerifyNonMembership(ctx, store, cdc, height, delayTimePeriod, delayBlockPeriod, proof, path)
}

func (cs ClientState) VerifyNextSequenceRecv(ctx sdk.Context, store sdk.KVStore, cdc codec.BinaryCodec, height exported.Height, delayTimePeriod uint64, delayBlockPeriod uint64, prefix exported.Prefix, proof []byte, portID string, channelID string, nextSequenceRecv uint64) error {
	merklePath := commitmenttypes.NewMerklePath(host.NextSequenceRecvPath(portID, channelID))
	merklePath, err := commitmenttypes.ApplyPrefix(prefix, merklePath)
	if err != nil {
		return err
	}
	path, err := cdc.Marshal(&merklePath)
	if err != nil {
		return err
	}
	return cs.VerifyMembershipWithContext(ctx, store, cdc, height, delayTimePeriod, delayBlockPeriod, proof, path, sdk.Uint64ToBigEndian(nextSequenceRecv))
}

func (cs ClientState) VerifyMembership(
	blockTime time.Time,
	clientStore sdk.KVStore,
	cdc codec.BinaryCodec,
	height exported.Height,
	delayTimePeriod uint64,
	delayBlockPeriod uint64,
	proof []byte,
	path []byte,
	value []byte,
) error {
	// NOTE: In future version of ibc-go, the prefix will be concatenated with the path and passed to the Client validation function.
	// https://github.com/cosmos/ibc-go/blob/088ba19bd451db152b881efd6f7bdf09f12a2171/modules/light-clients/07-tendermint/client_state.go#L205

	var merklePath commitmenttypes.MerklePath
	if err := cdc.Unmarshal(path, &merklePath); err != nil {
		return err
	}
	if l := len(merklePath.KeyPath); l != 2 {
		panic(fmt.Errorf("invalid KeyPath length: %v", l))
	}
	prefixBytes := []byte(merklePath.KeyPath[0])
	commitmentPath := []byte(merklePath.KeyPath[1])

	if cs.GetLatestHeight().LT(height) {
		return sdkerrors.Wrapf(
			sdkerrors.ErrInvalidHeight,
			"client state height < proof height (%d < %d), please ensure the client has been updated", cs.GetLatestHeight(), height,
		)
	}
	consensusState, err := GetConsensusState(clientStore, cdc, height)
	if err != nil {
		return sdkerrors.Wrapf(clienttypes.ErrConsensusStateNotFound, "please ensure the proof was constructed against a height that exists on the client: err=%v", err)
	}
	commitmentProof, err := ParseStateCommitmentProof(proof)
	if err != nil {
		return err
	}
	commitment, err := commitmentProof.GetCommitment()
	if err != nil {
		return err
	}
	if !height.EQ(commitment.Height) {
		return sdkerrors.Wrapf(ErrInvalidStateCommitment, "invalid height: expected=%v got=%v", height, commitment.Height)
	}
	if !bytes.Equal(prefixBytes, commitment.Prefix) {
		return sdkerrors.Wrapf(ErrInvalidStateCommitment, "invalid prefix: expected=%v got=%v", prefixBytes, commitment.Prefix)
	}
	if !bytes.Equal(commitmentPath, commitment.Path) {
		return sdkerrors.Wrapf(ErrInvalidStateCommitment, "invalid path: expected=%v got=%v", commitmentPath, commitment.Path)
	}
	if !bytes.Equal(value, commitment.Value) {
		return sdkerrors.Wrapf(ErrInvalidStateCommitment, "invalid value: expected=%v got=%v", value, commitment.Value)
	}
	if !commitment.StateID.EqualBytes(consensusState.StateId) {
		return sdkerrors.Wrapf(ErrInvalidStateCommitment, "invalid state ID: expected=%v got=%v", consensusState.StateId, commitment.StateID)
	}
	if err := VerifySignatureWithSignBytes(commitmentProof.CommitmentBytes, commitmentProof.Signature, commitmentProof.Signer); err != nil {
		return sdkerrors.Wrapf(ErrInvalidStateCommitmentProof, "failed to verify state commitment proof: %v", err)
	}
	signer := common.BytesToAddress(commitmentProof.Signer)
	if !cs.IsActiveKey(blockTime, signer) {
		return sdkerrors.Wrapf(ErrExpiredEnclaveKey, "key '%v' has expired", signer.Hex())
	}
	return nil
}

func (cs ClientState) VerifyMembershipWithContext(
	ctx sdk.Context,
	clientStore sdk.KVStore,
	cdc codec.BinaryCodec,
	height exported.Height,
	delayTimePeriod uint64,
	delayBlockPeriod uint64,
	proof []byte,
	path []byte,
	value []byte,
) error {
	if err := verifyDelayPeriodPassed(ctx, clientStore, height, delayTimePeriod, delayBlockPeriod); err != nil {
		return err
	}
	return cs.VerifyMembership(ctx.BlockTime(), clientStore, cdc, height, delayTimePeriod, delayBlockPeriod, proof, path, value)
}

// VerifyNonMembership is a generic proof verification method which verifies the absence of a given CommitmentPath at a specified height.
// The caller is expected to construct the full CommitmentPath from a CommitmentPrefix and a standardized path (as defined in ICS 24).
func (cs ClientState) VerifyNonMembership(
	ctx sdk.Context,
	clientStore sdk.KVStore,
	cdc codec.BinaryCodec,
	height exported.Height,
	delayTimePeriod uint64,
	delayBlockPeriod uint64,
	proof []byte,
	path []byte,
) error {
	if err := verifyDelayPeriodPassed(ctx, clientStore, height, delayTimePeriod, delayBlockPeriod); err != nil {
		return err
	}
	return cs.VerifyMembership(ctx.BlockTime(), clientStore, cdc, height, delayTimePeriod, delayBlockPeriod, proof, path, []byte{})
}

// verifyDelayPeriodPassed will ensure that at least delayTimePeriod amount of time and delayBlockPeriod number of blocks have passed
// since consensus state was submitted before allowing verification to continue.
func verifyDelayPeriodPassed(ctx sdk.Context, store sdk.KVStore, proofHeight exported.Height, delayTimePeriod, delayBlockPeriod uint64) error {
	if delayTimePeriod != 0 {
		// check that executing chain's timestamp has passed consensusState's processed time + delay time period
		processedTime, ok := GetProcessedTime(store, proofHeight)
		if !ok {
			return sdkerrors.Wrapf(ErrProcessedTimeNotFound, "processed time not found for height: %s", proofHeight)
		}

		currentTimestamp := uint64(ctx.BlockTime().UnixNano())
		validTime := processedTime + delayTimePeriod

		// NOTE: delay time period is inclusive, so if currentTimestamp is validTime, then we return no error
		if currentTimestamp < validTime {
			return sdkerrors.Wrapf(ErrDelayPeriodNotPassed, "cannot verify packet until time: %d, current time: %d",
				validTime, currentTimestamp)
		}

	}

	if delayBlockPeriod != 0 {
		// check that executing chain's height has passed consensusState's processed height + delay block period
		processedHeight, ok := GetProcessedHeight(store, proofHeight)
		if !ok {
			return sdkerrors.Wrapf(ErrProcessedHeightNotFound, "processed height not found for height: %s", proofHeight)
		}

		currentHeight := clienttypes.GetSelfHeight(ctx)
		validHeight := clienttypes.NewHeight(processedHeight.GetRevisionNumber(), processedHeight.GetRevisionHeight()+delayBlockPeriod)

		// NOTE: delay block period is inclusive, so if currentHeight is validHeight, then we return no error
		if currentHeight.LT(validHeight) {
			return sdkerrors.Wrapf(ErrDelayPeriodNotPassed, "cannot verify packet until height: %s, current height: %s",
				validHeight, currentHeight)
		}
	}

	return nil
}

// NOTE: At the moment, the verification function for IBC handshake does not have access to block time.
// Therefore, time.Now() is used just to pass the compile. This is of course unsafe.
// In the future ibc-go version, this issue will be resolved:
// https://github.com/cosmos/ibc-go/blob/088ba19bd451db152b881efd6f7bdf09f12a2171/modules/light-clients/07-tendermint/client_state.go
func fakeBlockTime() time.Time {
	return time.Now()
}
