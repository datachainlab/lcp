package types

import (
	sdkerrors "github.com/cosmos/cosmos-sdk/types/errors"
)

var (
	ErrInvalidStateCommitment      = sdkerrors.Register(ModuleName, 1, "invalid state commitment")
	ErrInvalidStateCommitmentProof = sdkerrors.Register(ModuleName, 2, "invalid state commitment proof")
	ErrExpiredEnclaveKey           = sdkerrors.Register(ModuleName, 3, "enclave key has expired")
	ErrProcessedTimeNotFound       = sdkerrors.Register(ModuleName, 4, "processed time not found")
	ErrProcessedHeightNotFound     = sdkerrors.Register(ModuleName, 5, "processed height not found")
	ErrDelayPeriodNotPassed        = sdkerrors.Register(ModuleName, 6, "packet-specified delay period has not been reached")
)
