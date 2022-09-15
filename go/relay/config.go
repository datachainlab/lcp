package relay

import (
	"encoding/hex"
	"fmt"
	"strings"

	codectypes "github.com/cosmos/cosmos-sdk/codec/types"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/hyperledger-labs/yui-relayer/core"
)

var _ core.ProverConfigI = (*ProverConfig)(nil)

var _ codectypes.UnpackInterfacesMessage = (*ProverConfig)(nil)

func (cfg *ProverConfig) UnpackInterfaces(unpacker codectypes.AnyUnpacker) error {
	if cfg == nil {
		return nil
	}
	if err := unpacker.UnpackAny(cfg.OriginProver, new(core.ProverConfigI)); err != nil {
		return err
	}
	return nil
}

func (pc ProverConfig) Build(chain core.ChainI) (core.ProverI, error) {
	if err := pc.Validate(); err != nil {
		return nil, err
	}
	prover, err := pc.OriginProver.GetCachedValue().(core.ProverConfigI).Build(chain)
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
	mrenclave, err := decodeMrenclaveHex(pc.Mrenclave)
	if err != nil {
		return err
	}
	if l := len(mrenclave); l != lcptypes.MrenclaveSize {
		return fmt.Errorf("MRENCLAVE length must be %v, but got %v", lcptypes.MrenclaveSize, l)
	}
	return nil
}

func decodeMrenclaveHex(s string) ([]byte, error) {
	s = strings.ToLower(strings.TrimPrefix(s, "0x"))
	return hex.DecodeString(s)
}
