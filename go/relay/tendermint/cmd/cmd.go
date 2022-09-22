package cmd

import (
	"github.com/cosmos/cosmos-sdk/codec"
	"github.com/hyperledger-labs/yui-relayer/config"
	"github.com/spf13/cobra"
)

func TendermintCmd(m codec.Codec, ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:   "lcp-tendermint",
		Short: "manage tendermint configurations",
	}

	cmd.AddCommand(
		configCmd(m),
		keysCmd(ctx),
		lightCmd(ctx),
	)

	return cmd
}
