package types

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"math/big"

	clienttypes "github.com/cosmos/ibc-go/v7/modules/core/02-client/types"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/common"
)

const (
	LCPCommitmentVersion          = 1
	LCPCommitmentTypeUpdateClient = 1
	LCPCommitmentTypeState        = 2
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
		{Name: "validation_params", Type: "bytes"},
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
	PrevStateID      *StateID
	NewStateID       StateID
	NewState         []byte
	PrevHeight       *clienttypes.Height
	NewHeight        clienttypes.Height
	Timestamp        *big.Int
	ValidationParams []byte
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
	Version    uint64
	Type       uint64
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
	// 0-7:   version
	// 8-15:  commitment type
	// 16-31: reserved
	version := binary.BigEndian.Uint64(p.Header[:8])
	commitmentType := binary.BigEndian.Uint64(p.Header[8:16])
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
		Timestamp        *big.Int `json:"timestamp"`
		ValidationParams []byte   `json:"validation_params"`
	})
	c := &UpdateClientCommitment{
		NewStateID:       p.NewStateId,
		NewState:         p.NewState,
		NewHeight:        clienttypes.Height{RevisionNumber: p.NewHeight.RevisionNumber, RevisionHeight: p.NewHeight.RevisionHeight},
		Timestamp:        p.Timestamp,
		ValidationParams: p.ValidationParams,
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
