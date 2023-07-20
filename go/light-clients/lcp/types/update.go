package types

import (
	"bytes"
	"time"

	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"
	clienttypes "github.com/cosmos/ibc-go/v7/modules/core/02-client/types"
	"github.com/cosmos/ibc-go/v7/modules/core/exported"
	"github.com/datachainlab/lcp/go/sgx/ias"
	mapset "github.com/deckarep/golang-set/v2"
	"github.com/ethereum/go-ethereum/common"
)

func (cs ClientState) VerifyClientMessage(ctx sdk.Context, cdc codec.BinaryCodec, clientStore sdk.KVStore, clientMsg exported.ClientMessage) error {
	switch clientMsg := clientMsg.(type) {
	case *UpdateClientHeader:
		return cs.verifyUpdateClient(ctx, cdc, clientStore, clientMsg)
	case *RegisterEnclaveKeyHeader:
		return cs.verifyRegisterEnclaveKey(ctx, cdc, clientStore, clientMsg)
	default:
		return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "unknown client message %T", clientMsg)
	}
}

func (cs ClientState) verifyUpdateClient(ctx sdk.Context, cdc codec.BinaryCodec, store sdk.KVStore, header *UpdateClientHeader) error {
	commitment, err := header.GetCommitment()
	if err != nil {
		return err
	}

	if cs.LatestHeight.IsZero() {
		if len(commitment.NewState) == 0 {
			return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid header %v: the commitment's `NewState` must be non-nil", header)
		}
	} else {
		if commitment.PrevHeight == nil || commitment.PrevStateID == nil {
			return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid header %v: the commitment's `PrevHeight` and `PrevStateID` must be non-nil", header)
		}
		prevConsensusState, err := GetConsensusState(store, cdc, commitment.PrevHeight)
		if err != nil {
			return err
		}
		if !bytes.Equal(prevConsensusState.StateId, commitment.PrevStateID[:]) {
			return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "unexpected StateID: expected=%v actual=%v", prevConsensusState.StateId, commitment.PrevStateID[:])
		}
	}

	signer := common.BytesToAddress(header.Signer)
	if !cs.IsActiveKey(ctx.BlockTime(), store, signer) {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "signer '%v' not found", signer)
	}

	if err := VerifySignatureWithSignBytes(header.Commitment, header.Signature, signer); err != nil {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, err.Error())
	}

	return nil
}

func (cs ClientState) verifyRegisterEnclaveKey(ctx sdk.Context, cdc codec.BinaryCodec, store sdk.KVStore, header *RegisterEnclaveKeyHeader) error {
	// TODO define error types

	if err := ias.VerifyReport(header.Report, header.Signature, header.SigningCert, ctx.BlockTime()); err != nil {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid header: header=%v, err=%v", header, err)
	}
	avr, err := ias.ParseAndValidateAVR(header.Report)
	if err != nil {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid AVR: report=%v err=%v", header.Report, err)
	}
	quoteStatus := avr.ISVEnclaveQuoteStatus.String()
	if quoteStatus == QuoteOK {
		if len(avr.AdvisoryIDs) != 0 {
			return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "advisory IDs should be empty when status is OK: actual=%v", avr.AdvisoryIDs)
		}
	} else {
		if !cs.isAllowedStatus(quoteStatus) {
			return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "disallowed quote status exists: allowed=%v actual=%v", cs.AllowedQuoteStatuses, quoteStatus)
		}
		if !cs.isAllowedAdvisoryIDs(avr.AdvisoryIDs) {
			return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "disallowed advisory ID(s) exists: allowed=%v actual=%v", cs.AllowedAdvisoryIds, avr.AdvisoryIDs)
		}
	}
	quote, err := avr.Quote()
	if err != nil {
		return err
	}
	if !bytes.Equal(cs.Mrenclave, quote.Report.MRENCLAVE[:]) {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid AVR: mrenclave mismatch: expected=%v actual=%v", cs.Mrenclave, quote.Report.MRENCLAVE[:])
	}
	addr, err := ias.GetEnclaveKeyAddress(quote)
	if err != nil {
		return err
	}
	if cs.Contains(store, addr) {
		return sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "signer '%v' already exists", addr.String())
	}
	return nil
}

func (cs ClientState) UpdateState(ctx sdk.Context, cdc codec.BinaryCodec, clientStore sdk.KVStore, clientMsg exported.ClientMessage) []exported.Height {
	switch clientMsg := clientMsg.(type) {
	case *UpdateClientHeader:
		return cs.updateClient(ctx, cdc, clientStore, clientMsg)
	case *RegisterEnclaveKeyHeader:
		return cs.registerEnclaveKey(ctx, cdc, clientStore, clientMsg)
	default:
		panic(sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "unknown client message %T", clientMsg))
	}
}

func (cs ClientState) updateClient(ctx sdk.Context, cdc codec.BinaryCodec, clientStore sdk.KVStore, header *UpdateClientHeader) []exported.Height {
	commitment, err := header.GetCommitment()
	if err != nil {
		panic(err)
	}
	if cs.LatestHeight.LT(commitment.NewHeight) {
		cs.LatestHeight = commitment.NewHeight
	}
	consensusState := ConsensusState{StateId: commitment.NewStateID[:], Timestamp: commitment.Timestamp.Uint64()}

	setClientState(clientStore, cdc, &cs)
	setConsensusState(clientStore, cdc, &consensusState, commitment.NewHeight)
	return nil
}

func (cs ClientState) registerEnclaveKey(ctx sdk.Context, cdc codec.BinaryCodec, clientStore sdk.KVStore, header *RegisterEnclaveKeyHeader) []exported.Height {
	avr, err := ias.ParseAndValidateAVR(header.Report)
	if err != nil {
		panic(sdkerrors.Wrapf(clienttypes.ErrInvalidHeader, "invalid AVR: report=%v err=%v", header.Report, err))
	}
	quote, err := avr.Quote()
	if err != nil {
		panic(err)
	}
	addr, err := ias.GetEnclaveKeyAddress(quote)
	if err != nil {
		panic(err)
	}
	cs.AddEnclaveKey(clientStore, addr, avr.GetTimestamp().Add(cs.getKeyExpiration()))
	return nil
}

func (cs ClientState) Contains(clientStore sdk.KVStore, key common.Address) bool {
	return clientStore.Has(enclaveKeyPath(key))
}

func (cs ClientState) IsActiveKey(blockTime time.Time, clientStore sdk.KVStore, key common.Address) bool {
	if !cs.Contains(clientStore, key) {
		return false
	}
	expiredAt := sdk.BigEndianToUint64(clientStore.Get(enclaveKeyPath(key)))
	return time.Unix(int64(expiredAt), 0).After(blockTime)
}

func (cs ClientState) AddEnclaveKey(clientStore sdk.KVStore, key common.Address, expiredAt time.Time) {
	clientStore.Set(enclaveKeyPath(key), sdk.Uint64ToBigEndian(uint64(expiredAt.Unix())))
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

func enclaveKeyPath(key common.Address) []byte {
	return []byte("aux/enclave_keys/" + key.Hex())
}
