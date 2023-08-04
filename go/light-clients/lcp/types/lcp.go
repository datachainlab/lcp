package types

import (
	"fmt"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/ethereum/go-ethereum/crypto/secp256k1"
)

const (
	QuoteOK                                = "OK"
	QuoteSignatureInvalid                  = "SIGNATURE_INVALID"
	QuoteGroupRevoked                      = "GROUP_REVOKED"
	QuoteSignatureRevoked                  = "SIGNATURE_REVOKED"
	QuoteKeyRevoked                        = "KEY_REVOKED"
	QuoteSigRLVersionMismatch              = "SIGRL_VERSION_MISMATCH"
	QuoteGroupOutOfDate                    = "GROUP_OUT_OF_DATE"
	QuoteConfigurationNeeded               = "CONFIGURATION_NEEDED"
	QuoteSwHardeningNeeded                 = "SW_HARDENING_NEEDED"
	QuoteConfigurationAndSwHardeningNeeded = "CONFIGURATION_AND_SW_HARDENING_NEEDED"
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

func VerifySignatureWithSignBytes(signBytes []byte, signature []byte, expectedSigner common.Address) error {
	if l := len(signature); l != 65 {
		return fmt.Errorf("invalid signature length: expected=%v actual=%v", 65, l)
	}
	if l := len(expectedSigner); l != common.AddressLength {
		return fmt.Errorf("invalid signer length: expected=%v actual=%v", common.AddressLength, l)
	}
	msg := crypto.Keccak256(signBytes)
	return VerifySignature(msg, signature, expectedSigner)
}
