package types

import (
	"bytes"
	"time"

	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	"github.com/cosmos/ibc-go/v4/modules/core/exported"
	"github.com/datachainlab/lcp/go/sgx/ias"
	"github.com/ethereum/go-ethereum/common"
)

// Update and Misbehaviour functions
func (cs ClientState) CheckHeaderAndUpdateState(ctx sdk.Context, cdc codec.BinaryCodec, store sdk.KVStore, header exported.Header) (exported.ClientState, exported.ConsensusState, error) {
	switch header := header.(type) {
	case *UpdateClientHeader:
		return cs.CheckHeaderAndUpdateForUpdateClient(ctx, cdc, store, header)
	case *RegisterEnclaveKeyHeader:
		return cs.CheckHeaderAndUpdateForRegisterEnclaveKey(ctx, cdc, store, header)
	default:
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "unknown header %T", header)
	}
}

func (cs ClientState) CheckHeaderAndUpdateForUpdateClient(ctx sdk.Context, cdc codec.BinaryCodec, store sdk.KVStore, header *UpdateClientHeader) (exported.ClientState, exported.ConsensusState, error) {
	commitment, err := header.GetCommitment()
	if err != nil {
		return nil, nil, err
	}

	switch {
	case commitment.PrevHeight == nil && commitment.PrevStateID == nil && !cs.LatestHeight.IsZero():
		// nop
	case commitment.PrevHeight != nil && commitment.PrevStateID != nil:
		prevConsensusState, err := GetConsensusState(store, cdc, commitment.PrevHeight)
		if err != nil {
			return nil, nil, err
		}
		if !bytes.Equal(prevConsensusState.StateId, commitment.PrevStateID[:]) {
			return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "unexpected StateID: expected=%v actual=%v", prevConsensusState.StateId, commitment.PrevStateID[:])
		}
	default:
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid header %T", header)
	}

	if !cs.Contains(header.Signer) {
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "signer %v already exists", header.Signer)
	}

	if err := VerifySignatureWithSignBytes(header.Commitment, header.Signature, header.Signer); err != nil {
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, err.Error())
	}

	if h := header.GetHeight().(clienttypes.Height); cs.LatestHeight.LT(h) {
		cs.LatestHeight = h
	}
	return &cs, &ConsensusState{StateId: commitment.NewStateID[:], Timestamp: commitment.Timestamp.Uint64()}, nil
}

func (cs ClientState) CheckHeaderAndUpdateForRegisterEnclaveKey(ctx sdk.Context, cdc codec.BinaryCodec, store sdk.KVStore, header *RegisterEnclaveKeyHeader) (exported.ClientState, exported.ConsensusState, error) {
	// TODO define error types

	if err := ias.VerifyReport(header.Report, header.Signature, header.SigningCert, ctx.BlockTime()); err != nil {
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid header %T", header)
	}
	avr, err := ias.ParseAndValidateAVR(header.Report)
	if err != nil {
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid AVR %v", header.Report)
	}
	quote, err := avr.Quote()
	if err != nil {
		return nil, nil, err
	}
	addr, err := ias.GetEnclaveKeyAddress(quote)
	if err != nil {
		return nil, nil, err
	}

	newClientState := cs.WithNewKey(addr, avr.GetTimestamp())
	return &newClientState, &ConsensusState{}, nil
}

func (cs ClientState) Contains(signer []byte) bool {
	for _, key := range cs.Keys {
		if bytes.Equal(signer, key) {
			return true
		}
	}
	return false
}

func (cs ClientState) IsActiveKey(currentTime time.Time, signer common.Address) bool {
	expiredTime := uint64(currentTime.Add(-cs.getKeyExpiration()).Unix())
	for i, key := range cs.Keys {
		if bytes.Equal(signer[:], key) {
			// TODO remove key if it's expired
			return cs.AttestationTimes[i] > expiredTime
		}
	}
	return false
}

func (cs ClientState) WithNewKey(signer common.Address, attestationTime time.Time) ClientState {
	cs.Keys = append(cs.Keys, signer[:])
	cs.AttestationTimes = append(cs.AttestationTimes, uint64(attestationTime.Unix()))
	return cs
}

func (cs ClientState) getKeyExpiration() time.Duration {
	return time.Duration(cs.KeyExpiration) * time.Second
}
