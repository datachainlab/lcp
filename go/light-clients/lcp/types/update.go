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
	mapset "github.com/deckarep/golang-set/v2"
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

	if cs.LatestHeight.IsZero() {
		if len(commitment.NewState) == 0 {
			return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid header %v: the commitment's `NewState` must be non-nil", header)
		}
	} else {
		if commitment.PrevHeight == nil || commitment.PrevStateID == nil {
			return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid header %v: the commitment's `PrevHeight` and `PrevStateID` must be non-nil", header)
		}
		prevConsensusState, err := GetConsensusState(store, cdc, commitment.PrevHeight)
		if err != nil {
			return nil, nil, err
		}
		if !bytes.Equal(prevConsensusState.StateId, commitment.PrevStateID[:]) {
			return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "unexpected StateID: expected=%v actual=%v", prevConsensusState.StateId, commitment.PrevStateID[:])
		}
	}

	signer := common.BytesToAddress(header.Signer)
	if !cs.IsActiveKey(ctx.BlockTime(), signer) {
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "signer '%v' not found", signer)
	}

	if err := VerifySignatureWithSignBytes(header.Commitment, header.Signature, signer); err != nil {
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
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid header: header=%v, err=%v", header, err)
	}
	avr, err := ias.ParseAndValidateAVR(header.Report)
	if err != nil {
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid AVR: report=%v err=%v", header.Report, err)
	}
	quoteStatus := avr.ISVEnclaveQuoteStatus.String()
	if quoteStatus == QuoteOK {
		if len(avr.AdvisoryIDs) != 0 {
			return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "advisory IDs should be empty when status is OK: actual=%v", avr.AdvisoryIDs)
		}
	} else {
		if !cs.isAllowedStatus(quoteStatus) {
			return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "disallowed quote status exists: allowed=%v actual=%v", cs.AllowedQuoteStatuses, quoteStatus)
		}
		if !cs.isAllowedAdvisoryIDs(avr.AdvisoryIDs) {
			return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "disallowed advisory ID(s) exists: allowed=%v actual=%v", cs.AllowedAdvisoryIds, avr.AdvisoryIDs)
		}
	}
	quote, err := avr.Quote()
	if err != nil {
		return nil, nil, err
	}
	if !bytes.Equal(cs.Mrenclave, quote.Report.MRENCLAVE[:]) {
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid AVR: mrenclave mismatch: expected=%v actual=%v", cs.Mrenclave, quote.Report.MRENCLAVE[:])
	}
	addr, err := ias.GetEnclaveKeyAddress(quote)
	if err != nil {
		return nil, nil, err
	}
	if cs.Contains(addr[:]) {
		return nil, nil, sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "signer '%v' already exists", addr.String())
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

func (cs ClientState) isAllowedStatus(status string) bool {
	if status == QuoteOK {
		return true
	}
	for _, s := range cs.AllowedQuoteStatuses {
		if status == s {
			return true
		}
	}
	return false
}

func (cs ClientState) isAllowedAdvisoryIDs(advIDs []string) bool {
	if len(advIDs) == 0 {
		return true
	}
	set := mapset.NewThreadUnsafeSet(cs.AllowedAdvisoryIds...)
	return set.Contains(advIDs...)
}
