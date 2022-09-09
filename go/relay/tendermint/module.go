package module

import (
	codectypes "github.com/cosmos/cosmos-sdk/codec/types"
	"github.com/datachainlab/lcp/go/relay/tendermint/cmd"
	"github.com/hyperledger-labs/yui-relayer/config"
	"github.com/spf13/cobra"
)

type Module struct{}

var _ config.ModuleI = (*Module)(nil)

// Name returns the name of the module
func (m Module) Name() string {
	return "lcp-tendermint"
}

// RegisterInterfaces register the module interfaces to protobuf Any.
func (m Module) RegisterInterfaces(registry codectypes.InterfaceRegistry) {}

// GetCmd returns the command
func (m Module) GetCmd(ctx *config.Context) *cobra.Command {
	return cmd.TendermintCmd(ctx.Codec, ctx)
}
