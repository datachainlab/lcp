package types

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"math/big"
	"time"

	clienttypes "github.com/cosmos/ibc-go/v7/modules/core/02-client/types"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

const (
	LCPCommitmentVersion          = 1
	LCPCommitmentTypeUpdateClient = 1
	LCPCommitmentTypeState        = 2
)

const (
	LCPCommitmentContextTypeEmpty          = 0
	LCPCommitmentContextTypeTrustingPeriod = 1
)

var (
	commitmentProofABI, _ = abi.NewType("tuple", "struct CommitmentProof", []abi.ArgumentMarshaling{
		{Name: "commitment_bytes", Type: "bytes"},
		{Name: "signer", Type: "address"},
		{Name: "signature", Type: "bytes"},
	})

	headeredCommitmentABI, _ = abi.NewType("tuple", "struct HeaderedCommitment", []abi.ArgumentMarshaling{
		{Name: "header", Type: "bytes32"},
		{Name: "commitment", Type: "bytes"},
	})

	updateClientCommitmentABI, _ = abi.NewType("tuple", "struct UpdateClientCommitment", []abi.ArgumentMarshaling{
		{Name: "prev_state_id", Type: "bytes32"},
		{Name: "new_state_id", Type: "bytes32"},
		{Name: "new_state", Type: "bytes"},
		{Name: "prev_height", Type: "tuple", Components: []abi.ArgumentMarshaling{
			{Name: "revision_number", Type: "uint64"},
			{Name: "revision_height", Type: "uint64"},
		}},
		{Name: "new_height", Type: "tuple", Components: []abi.ArgumentMarshaling{
			{Name: "revision_number", Type: "uint64"},
			{Name: "revision_height", Type: "uint64"},
		}},
		{Name: "timestamp", Type: "uint128"},
		{Name: "context", Type: "bytes"},
	})

	headeredCommitmentContextABI, _ = abi.NewType("tuple", "struct HeaderedCommitmentContext", []abi.ArgumentMarshaling{
		{Name: "header", Type: "bytes32"},
		{Name: "context_bytes", Type: "bytes"},
	})

	trustingPeriodContextABI, _ = abi.NewType("tuple", "struct TrustingPeriodCommitmentContext", []abi.ArgumentMarshaling{
		{Name: "params", Type: "bytes32"},
		{Name: "timestamps", Type: "bytes32"},
	})

	stateCommitmentABI, _ = abi.NewType("tuple", "struct StateCommitment", []abi.ArgumentMarshaling{
		{Name: "prefix", Type: "bytes"},
		{Name: "path", Type: "bytes"},
		{Name: "value", Type: "bytes32"},
		{Name: "height", Type: "tuple", Components: []abi.ArgumentMarshaling{
			{Name: "revision_number", Type: "uint64"},
			{Name: "revision_height", Type: "uint64"},
		}},
		{Name: "state_id", Type: "bytes32"},
	})
)

type StateID [32]byte

func (id StateID) EqualBytes(bz []byte) bool {
	return bytes.Equal(id[:], bz)
}

type UpdateClientCommitment struct {
	PrevStateID *StateID
	NewStateID  StateID
	NewState    []byte
	PrevHeight  *clienttypes.Height
	NewHeight   clienttypes.Height
	Timestamp   *big.Int
	Context     CommitmentContext
}

// CommitmentContext is the interface for the context of a commitment.
type CommitmentContext interface {
	Validate(time.Time) error
}

// NoneCommitmentContext is the commitment context for a commitment that does not require any validation.
type NoneCommitmentContext struct{}

var _ CommitmentContext = NoneCommitmentContext{}

func (NoneCommitmentContext) Validate(time.Time) error {
	return nil
}

// TrustingPeriodCommitmentContext is the commitment context for a commitment that requires the current time to be within the trusting period.
type TrustingPeriodCommitmentContext struct {
	TrustingPeriod           big.Int
	ClockDrift               big.Int
	UntrustedHeaderTimestamp time.Time
	TrustedStateTimestamp    time.Time
}

func DecodeTrustingPeriodCommitmentContext(params, timestamps [32]byte) *TrustingPeriodCommitmentContext {
	// MSB first
	// 0-15: trusting_period
	// 16-31: clock_drift
	trustingPeriod := uint128BytesToBigInt(params[:16])
	clockDrift := uint128BytesToBigInt(params[16:32])

	// 0-15: untrusted_header_timestamp
	// 16-31: trusted_state_timestamp
	return &TrustingPeriodCommitmentContext{
		TrustingPeriod:           trustingPeriod,
		ClockDrift:               clockDrift,
		UntrustedHeaderTimestamp: timestampNanosBytesToTime(timestamps[:16]),
		TrustedStateTimestamp:    timestampNanosBytesToTime(timestamps[16:32]),
	}
}

func uint128BytesToBigInt(bz []byte) big.Int {
	if len(bz) != 16 {
		panic("invalid length")
	}
	var durationNanos big.Int
	durationNanos.SetBytes(bz)
	return durationNanos
}

func timestampNanosBytesToTime(bz []byte) time.Time {
	if len(bz) != 16 {
		panic("invalid length")
	}
	var (
		timestampNanos big.Int
		secs           big.Int
		nanos          big.Int
	)

	timestampNanos.SetBytes(bz)
	secs.Div(&timestampNanos, big.NewInt(1e9))
	nanos.Mod(&timestampNanos, big.NewInt(1e9))
	return time.Unix(secs.Int64(), nanos.Int64())
}

var _ CommitmentContext = TrustingPeriodCommitmentContext{}

func timeToBigInt(t time.Time) big.Int {
	var (
		secs  big.Int
		nanos big.Int
	)
	secs.SetInt64(t.Unix())
	secs.Mul(&secs, big.NewInt(1e9))
	nanos.SetInt64(int64(t.Nanosecond()))
	secs.Add(&secs, &nanos)
	return secs
}

func (c TrustingPeriodCommitmentContext) Validate(now time.Time) error {
	currentTimestamp := timeToBigInt(now)
	trustedStateTimestamp := timeToBigInt(c.TrustedStateTimestamp)
	untrustedHeaderTimestamp := timeToBigInt(c.UntrustedHeaderTimestamp)

	var (
		trustingPeriodEnd       big.Int
		driftedCurrentTimestamp big.Int
	)
	trustingPeriodEnd.Add(&trustedStateTimestamp, &c.TrustingPeriod)
	driftedCurrentTimestamp.Add(&currentTimestamp, &c.ClockDrift)

	// ensure current timestamp is within trusting period
	if currentTimestamp.Cmp(&trustingPeriodEnd) > 0 {
		return fmt.Errorf("current time is after trusting period end: trusting_period_end=%v current=%v trusted_state_timestamp=%v trusting_period=%v", trustingPeriodEnd, now, c.TrustedStateTimestamp, c.TrustingPeriod)
	}
	// ensure header's timestamp indicates past
	if untrustedHeaderTimestamp.Cmp(&driftedCurrentTimestamp) > 0 {
		return fmt.Errorf("untrusted header timestamp is after current time: untrusted_header_timestamp=%v current=%v clock_drift=%v", c.UntrustedHeaderTimestamp, driftedCurrentTimestamp, c.ClockDrift)
	}
	return nil
}

type StateCommitment struct {
	Prefix  []byte
	Path    []byte
	Value   [32]byte
	Height  clienttypes.Height
	StateID StateID
}

type CommitmentProof struct {
	CommitmentBytes []byte
	Signer          common.Address
	Signature       []byte
}

func (p CommitmentProof) GetCommitment() (*HeaderedCommitment, error) {
	return EthABIDecodeHeaderedCommitment(p.CommitmentBytes)
}

type HeaderedCommitment struct {
	Version    uint16
	Type       uint16
	Commitment []byte
}

func (c HeaderedCommitment) GetUpdateClientCommitment() (*UpdateClientCommitment, error) {
	if c.Version != LCPCommitmentVersion {
		return nil, fmt.Errorf("unexpected commitment version: expected=%v actual=%v", LCPCommitmentVersion, c.Version)
	}
	if c.Type != LCPCommitmentTypeUpdateClient {
		return nil, fmt.Errorf("unexpected commitment type: expected=%v actual=%v", LCPCommitmentTypeUpdateClient, c.Type)
	}
	return EthABIDecodeUpdateClientCommitment(c.Commitment)
}

func (c HeaderedCommitment) GetStateCommitment() (*StateCommitment, error) {
	if c.Version != LCPCommitmentVersion {
		return nil, fmt.Errorf("unexpected commitment version: expected=%v actual=%v", LCPCommitmentVersion, c.Version)
	}
	if c.Type != LCPCommitmentTypeState {
		return nil, fmt.Errorf("unexpected commitment type: expected=%v actual=%v", LCPCommitmentTypeState, c.Type)
	}
	return EthABIDecodeStateCommitment(c.Commitment)
}

func EthABIEncodeCommitmentProof(p *CommitmentProof) ([]byte, error) {
	packer := abi.Arguments{
		{Type: commitmentProofABI},
	}
	return packer.Pack(p)
}

func EthABIDecodeCommitmentProof(bz []byte) (*CommitmentProof, error) {
	unpacker := abi.Arguments{
		{Type: commitmentProofABI},
	}
	v, err := unpacker.Unpack(bz)
	if err != nil {
		return nil, err
	}
	p := CommitmentProof(v[0].(struct {
		CommitmentBytes []byte         `json:"commitment_bytes"`
		Signer          common.Address `json:"signer"`
		Signature       []byte         `json:"signature"`
	}))
	return &p, nil
}

func EthABIDecodeHeaderedCommitment(bz []byte) (*HeaderedCommitment, error) {
	unpacker := abi.Arguments{
		{Type: headeredCommitmentABI},
	}
	v, err := unpacker.Unpack(bz)
	if err != nil {
		return nil, err
	}
	p := v[0].(struct {
		Header     [32]byte `json:"header"`
		Commitment []byte   `json:"commitment"`
	})
	// Header format:
	// MSB first
	// 0-1:  version
	// 2-3:  commitment type
	// 4-31: reserved
	version := binary.BigEndian.Uint16(p.Header[:2])
	commitmentType := binary.BigEndian.Uint16(p.Header[2:4])
	return &HeaderedCommitment{
		Version:    version,
		Type:       commitmentType,
		Commitment: p.Commitment,
	}, nil
}

func EthABIDecodeUpdateClientCommitment(bz []byte) (*UpdateClientCommitment, error) {
	unpacker := abi.Arguments{
		{Type: updateClientCommitmentABI},
	}
	v, err := unpacker.Unpack(bz)
	if err != nil {
		return nil, err
	}
	p := v[0].(struct {
		PrevStateId [32]byte `json:"prev_state_id"`
		NewStateId  [32]byte `json:"new_state_id"`
		NewState    []byte   `json:"new_state"`
		PrevHeight  struct {
			RevisionNumber uint64 `json:"revision_number"`
			RevisionHeight uint64 `json:"revision_height"`
		} `json:"prev_height"`
		NewHeight struct {
			RevisionNumber uint64 `json:"revision_number"`
			RevisionHeight uint64 `json:"revision_height"`
		} `json:"new_height"`
		Timestamp *big.Int `json:"timestamp"`
		Context   []byte   `json:"context"`
	})
	cctx, err := EthABIDecodeCommitmentContext(p.Context)
	if err != nil {
		return nil, err
	}
	c := &UpdateClientCommitment{
		NewStateID: p.NewStateId,
		NewState:   p.NewState,
		NewHeight:  clienttypes.Height{RevisionNumber: p.NewHeight.RevisionNumber, RevisionHeight: p.NewHeight.RevisionHeight},
		Timestamp:  p.Timestamp,
		Context:    cctx,
	}
	if p.PrevStateId != [32]byte{} {
		prev := StateID(p.PrevStateId)
		c.PrevStateID = &prev
	}
	if p.PrevHeight.RevisionNumber != 0 || p.PrevHeight.RevisionHeight != 0 {
		c.PrevHeight = &clienttypes.Height{RevisionNumber: p.PrevHeight.RevisionNumber, RevisionHeight: p.PrevHeight.RevisionHeight}
	}
	return c, nil
}

func EthABIDecodeCommitmentContext(bz []byte) (CommitmentContext, error) {
	unpacker := abi.Arguments{
		{Type: headeredCommitmentContextABI},
	}
	v, err := unpacker.Unpack(bz)
	if err != nil {
		return nil, err
	}
	p := v[0].(struct {
		Header       [32]byte `json:"header"`
		ContextBytes []byte   `json:"context_bytes"`
	})
	// Header format:
	// MSB first
	// 0-1:  type
	// 2-31: reserved
	contextType := binary.BigEndian.Uint16(p.Header[:2])
	switch contextType {
	case LCPCommitmentContextTypeEmpty:
		if len(p.ContextBytes) != 0 {
			return nil, fmt.Errorf("unexpected context bytes for empty commitment context: %X", p.ContextBytes)
		}
		return &NoneCommitmentContext{}, nil
	case LCPCommitmentContextTypeTrustingPeriod:
		return EthABIDecodeTrustingPeriodCommitmentContext(p.ContextBytes)
	default:
		return nil, fmt.Errorf("unexpected commitment context type: %v", contextType)
	}
}

func EthABIDecodeTrustingPeriodCommitmentContext(bz []byte) (*TrustingPeriodCommitmentContext, error) {
	if len(bz) != 64 {
		return nil, fmt.Errorf("unexpected length of trusting period commitment context: %d", len(bz))
	}
	unpacker := abi.Arguments{
		{Type: trustingPeriodContextABI},
	}
	v, err := unpacker.Unpack(bz)
	if err != nil {
		return nil, err
	}
	p := v[0].(struct {
		Params     [32]byte `json:"params"`
		Timestamps [32]byte `json:"timestamps"`
	})
	return DecodeTrustingPeriodCommitmentContext(p.Params, p.Timestamps), nil
}

func EthABIDecodeStateCommitment(bz []byte) (*StateCommitment, error) {
	unpacker := abi.Arguments{
		{Type: stateCommitmentABI},
	}
	v, err := unpacker.Unpack(bz)
	if err != nil {
		return nil, err
	}
	p := v[0].(struct {
		Prefix []byte   `json:"prefix"`
		Path   []byte   `json:"path"`
		Value  [32]byte `json:"value"`
		Height struct {
			RevisionNumber uint64 `json:"revision_number"`
			RevisionHeight uint64 `json:"revision_height"`
		} `json:"height"`
		StateId [32]byte `json:"state_id"`
	})
	return &StateCommitment{
		Prefix:  p.Prefix,
		Path:    p.Path,
		Value:   p.Value,
		Height:  clienttypes.Height{RevisionNumber: p.Height.RevisionNumber, RevisionHeight: p.Height.RevisionHeight},
		StateID: StateID(p.StateId),
	}, nil
}
