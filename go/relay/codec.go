package relay

import (
	codectypes "github.com/cosmos/cosmos-sdk/codec/types"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/hyperledger-labs/yui-relayer/core"
)

// RegisterInterfaces register the module interfaces to protobuf Any.
func RegisterInterfaces(registry codectypes.InterfaceRegistry) {
	lcptypes.RegisterInterfaces(registry)
	registry.RegisterImplementations(
		(*core.ProverConfig)(nil),
		&ProverConfig{},
	)
}
