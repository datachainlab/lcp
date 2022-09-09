package relay

import (
	"encoding/hex"
	"strings"

	codectypes "github.com/cosmos/cosmos-sdk/codec/types"
	"github.com/hyperledger-labs/yui-relayer/core"
)

var _ core.ProverConfigI = (*ProverConfig)(nil)

var _ codectypes.UnpackInterfacesMessage = (*ProverConfig)(nil)

func (cfg *ProverConfig) UnpackInterfaces(unpacker codectypes.AnyUnpacker) error {
	if cfg == nil {
		return nil
	}
	if err := unpacker.UnpackAny(cfg.UpstreamProver, new(core.ProverConfigI)); err != nil {
		return err
	}
	return nil
}

func (pc ProverConfig) Build(chain core.ChainI) (core.ProverI, error) {
	if err := pc.Validate(); err != nil {
		return nil, err
	}
	prover, err := pc.UpstreamProver.GetCachedValue().(core.ProverConfigI).Build(chain)
	if err != nil {
		return nil, err
	}
	return NewProver(pc, chain, prover)
}

func (pc ProverConfig) GetMrenclave() []byte {
	mrenclave, err := decodeMrenclaveHex(pc.Mrenclave)
	if err != nil {
		panic(err)
	}
	return mrenclave
}

func (pc ProverConfig) Validate() error {
	_, err := decodeMrenclaveHex(pc.Mrenclave)
	if err != nil {
		return err
	}
	return nil
}

func decodeMrenclaveHex(s string) ([]byte, error) {
	s = strings.ToLower(strings.TrimPrefix(s, "0x"))
	return hex.DecodeString(s)
}
