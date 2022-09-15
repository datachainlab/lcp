package types

import (
	"bytes"
	"fmt"
	"math/big"

	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/crypto/secp256k1"
	"github.com/ethereum/go-ethereum/rlp"
)

// Note: currently, LCP supports only secp256k1

func VerifySignature(msg []byte, signature []byte, signer common.Address) error {
	pubKey, err := secp256k1.RecoverPubkey(msg, signature)
	if err != nil {
		return err
	}
	pub, err := crypto.UnmarshalPubkey(pubKey)
	if err != nil {
		return err
	}
	addr := crypto.PubkeyToAddress(*pub)
	if signer == addr {
		return nil
	} else {
		return fmt.Errorf("unexpected signer: expected=%v actual=%v", signer.Hex(), addr.Hex())
	}
}

func VerifySignatureWithSignBytes(signBytes []byte, signature []byte, expectedSigner []byte) error {
	if l := len(signature); l != 65 {
		return fmt.Errorf("invalid signature length: expected=%v actual=%v", 65, l)
	}
	if l := len(expectedSigner); l != common.AddressLength {
		return fmt.Errorf("invalid signer length: expected=%v actual=%v", common.AddressLength, l)
	}
	msg := crypto.Keccak256(signBytes)
	return VerifySignature(msg, signature, common.BytesToAddress(expectedSigner))
}

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
	Timestamp        big.Int
	ValidationParams []byte
}

type RLPUpdateClientCommitment struct {
	PrevStateID      []byte
	NewStateID       [32]byte
	NewState         []byte
	PrevHeight       []byte
	NewHeight        [16]byte
	Timestamp        [16]byte
	ValidationParams []byte
}

func (c *RLPUpdateClientCommitment) ToUpdateClientCommitment() (*UpdateClientCommitment, error) {
	var (
		commitment UpdateClientCommitment
		err        error
	)
	if l := len(c.PrevStateID); l == 32 {
		commitment.PrevStateID = (*StateID)(c.PrevStateID)
	} else if l > 0 {
		return nil, fmt.Errorf("PrevStateID length must be 0 or 32, but got %v", l)
	}
	commitment.NewStateID = c.NewStateID
	commitment.NewState = c.NewState
	if len(c.PrevHeight) > 0 {
		commitment.PrevHeight, err = bzToHeight(c.PrevHeight)
		if err != nil {
			return nil, err
		}
	}
	newHeight, err := bzToHeight(c.NewHeight[:])
	if err != nil {
		return nil, err
	}
	commitment.NewHeight = *newHeight
	var timestamp big.Int
	commitment.Timestamp = *timestamp.SetBytes(c.Timestamp[:])
	commitment.ValidationParams = c.ValidationParams
	return &commitment, nil
}

func bzToHeight(bz []byte) (*clienttypes.Height, error) {
	var height clienttypes.Height
	if l := len(bz); l != 16 {
		return nil, fmt.Errorf("unexpcted bytes length: expected=%v actual=%v", 16, l)
	}
	height.RevisionNumber = sdk.BigEndianToUint64(bz[:8])
	height.RevisionHeight = sdk.BigEndianToUint64(bz[8:])
	return &height, nil
}

func ParseUpdateClientCommitment(bz []byte) (*UpdateClientCommitment, error) {
	var rc RLPUpdateClientCommitment
	if err := rlp.DecodeBytes(bz, &rc); err != nil {
		return nil, err
	}
	return rc.ToUpdateClientCommitment()
}

type StateCommitment struct {
	Prefix  []byte
	Path    []byte
	Value   []byte
	Height  clienttypes.Height
	StateID StateID
}

type StateCommitmentProof struct {
	CommitmentBytes []byte
	Signer          []byte
	Signature       []byte
}

func (p StateCommitmentProof) ToRLPBytes() []byte {
	bz, err := rlp.EncodeToBytes(p)
	if err != nil {
		panic(err)
	}
	return bz
}

func (p StateCommitmentProof) GetCommitment() (*StateCommitment, error) {
	return ParseStateCommitment(p.CommitmentBytes)
}

func ParseStateCommitmentProof(bz []byte) (*StateCommitmentProof, error) {
	var proof StateCommitmentProof
	if err := rlp.DecodeBytes(bz, &proof); err != nil {
		return nil, err
	}
	return &proof, nil
}

func ParseStateCommitment(bz []byte) (*StateCommitment, error) {
	var commitment StateCommitment
	buf := bytes.NewBuffer(bz)
	st := rlp.NewStream(buf, 0)
	if _, err := st.List(); err != nil {
		return nil, err
	}
	if err := st.Decode(&commitment.Prefix); err != nil {
		return nil, err
	}
	if err := st.Decode(&commitment.Path); err != nil {
		return nil, err
	}
	if err := st.Decode(&commitment.Value); err != nil {
		return nil, err
	}
	heightBytes, err := st.Bytes()
	if err != nil {
		return nil, err
	}
	height, err := bzToHeight(heightBytes)
	if err != nil {
		return nil, err
	}
	commitment.Height = *height
	if err := st.Decode(&commitment.StateID); err != nil {
		return nil, err
	}
	if err := st.ListEnd(); err != nil {
		return nil, err
	}
	return &commitment, nil
}
