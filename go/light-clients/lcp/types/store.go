package types

import (
	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"
	clienttypes "github.com/cosmos/ibc-go/v7/modules/core/02-client/types"
	host "github.com/cosmos/ibc-go/v7/modules/core/24-host"
	"github.com/cosmos/ibc-go/v7/modules/core/exported"
)

var (
	// KeyProcessedTime is appended to consensus state key to store the processed time
	KeyProcessedTime = []byte("/processedTime")
	// KeyProcessedHeight is appended to consensus state key to store the processed height
	KeyProcessedHeight = []byte("/processedHeight")
)

// setClientState stores the client state
func setClientState(clientStore sdk.KVStore, cdc codec.BinaryCodec, clientState *ClientState) {
	key := host.ClientStateKey()
	val := clienttypes.MustMarshalClientState(cdc, clientState)
	clientStore.Set(key, val)
}

// setConsensusState stores the consensus state at the given height.
func setConsensusState(clientStore sdk.KVStore, cdc codec.BinaryCodec, consensusState *ConsensusState, height exported.Height) {
	key := host.ConsensusStateKey(height)
	val := clienttypes.MustMarshalConsensusState(cdc, consensusState)
	clientStore.Set(key, val)
}

// GetConsensusState retrieves the consensus state from the client prefixed
// store. An error is returned if the consensus state does not exist.
func GetConsensusState(store sdk.KVStore, cdc codec.BinaryCodec, height exported.Height) (*ConsensusState, error) {
	bz := store.Get(host.ConsensusStateKey(height))
	if bz == nil {
		return nil, sdkerrors.Wrapf(
			clienttypes.ErrConsensusStateNotFound,
			"consensus state does not exist for height %s", height,
		)
	}

	consensusStateI, err := clienttypes.UnmarshalConsensusState(cdc, bz)
	if err != nil {
		return nil, sdkerrors.Wrapf(clienttypes.ErrInvalidConsensus, "unmarshal error: %v", err)
	}

	consensusState, ok := consensusStateI.(*ConsensusState)
	if !ok {
		return nil, sdkerrors.Wrapf(
			clienttypes.ErrInvalidConsensus,
			"invalid consensus type %T, expected %T", consensusState, &ConsensusState{},
		)
	}

	return consensusState, nil
}

// ProcessedTimeKey returns the key under which the processed time will be stored in the client store.
func ProcessedTimeKey(height exported.Height) []byte {
	return append(host.ConsensusStateKey(height), KeyProcessedTime...)
}

// SetProcessedTime stores the time at which a header was processed and the corresponding consensus state was created.
// This is useful when validating whether a packet has reached the time specified delay period in the tendermint client's
// verification functions
func SetProcessedTime(clientStore sdk.KVStore, height exported.Height, timeNs uint64) {
	key := ProcessedTimeKey(height)
	val := sdk.Uint64ToBigEndian(timeNs)
	clientStore.Set(key, val)
}

// GetProcessedTime gets the time (in nanoseconds) at which this chain received and processed a tendermint header.
// This is used to validate that a received packet has passed the time delay period.
func GetProcessedTime(clientStore sdk.KVStore, height exported.Height) (uint64, bool) {
	key := ProcessedTimeKey(height)
	bz := clientStore.Get(key)
	if bz == nil {
		return 0, false
	}
	return sdk.BigEndianToUint64(bz), true
}

// deleteProcessedTime deletes the processedTime for a given height
func deleteProcessedTime(clientStore sdk.KVStore, height exported.Height) {
	key := ProcessedTimeKey(height)
	clientStore.Delete(key)
}

// ProcessedHeightKey returns the key under which the processed height will be stored in the client store.
func ProcessedHeightKey(height exported.Height) []byte {
	return append(host.ConsensusStateKey(height), KeyProcessedHeight...)
}

// SetProcessedHeight stores the height at which a header was processed and the corresponding consensus state was created.
// This is useful when validating whether a packet has reached the specified block delay period in the tendermint client's
// verification functions
func SetProcessedHeight(clientStore sdk.KVStore, consHeight, processedHeight exported.Height) {
	key := ProcessedHeightKey(consHeight)
	val := []byte(processedHeight.String())
	clientStore.Set(key, val)
}

// GetProcessedHeight gets the height at which this chain received and processed a tendermint header.
// This is used to validate that a received packet has passed the block delay period.
func GetProcessedHeight(clientStore sdk.KVStore, height exported.Height) (exported.Height, bool) {
	key := ProcessedHeightKey(height)
	bz := clientStore.Get(key)
	if bz == nil {
		return nil, false
	}
	processedHeight, err := clienttypes.ParseHeight(string(bz))
	if err != nil {
		return nil, false
	}
	return processedHeight, true
}

// deleteProcessedHeight deletes the processedHeight for a given height
func deleteProcessedHeight(clientStore sdk.KVStore, height exported.Height) {
	key := ProcessedHeightKey(height)
	clientStore.Delete(key)
}
