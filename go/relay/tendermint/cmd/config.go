package cmd

import (
	"encoding/json"
	"fmt"

	"github.com/cosmos/cosmos-sdk/codec"
	"github.com/hyperledger-labs/yui-relayer/chains/tendermint"
	"github.com/hyperledger-labs/yui-relayer/core"
	"github.com/spf13/cobra"
)

func configCmd(m codec.Codec) *cobra.Command {
	cmd := &cobra.Command{
		Use:   "config",
		Short: "manage configuration file",
	}

	cmd.AddCommand(
		generateChainConfigCmd(m),
	)

	return cmd
}

func generateChainConfigCmd(m codec.Codec) *cobra.Command {
	cmd := &cobra.Command{
		Use:  "generate",
		Args: cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) (err error) {
			// TODO make it configurable
			c := tendermint.ChainConfig{
				Key:           "testkey",
				ChainId:       args[0],
				RpcAddr:       "http://localhost:26557",
				AccountPrefix: "cosmos",
				GasAdjustment: 1.5,
				GasPrices:     "0.025stake",
			}
			p := tendermint.ProverConfig{
				TrustingPeriod: "336h",
			}
			config, err := core.NewChainProverConfig(m, &c, &p)
			if err != nil {
				return err
			}
			bz, err := json.Marshal(config)
			if err != nil {
				return err
			}
			fmt.Println(string(bz))
			return nil
		},
	}
	return cmd
}
