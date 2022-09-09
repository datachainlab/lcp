package cmd

import (
	"fmt"
	"strconv"

	"github.com/cosmos/cosmos-sdk/client/flags"
	tmclient "github.com/cosmos/ibc-go/v4/modules/light-clients/07-tendermint/types"
	"github.com/datachainlab/lcp/go/relay"
	"github.com/hyperledger-labs/yui-relayer/chains/tendermint"
	"github.com/hyperledger-labs/yui-relayer/config"
	"github.com/spf13/cobra"
)

// chainCmd represents the keys command
func lightCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:     "light",
		Aliases: []string{"l"},
		Short:   "manage light clients held by the relayer for each chain",
	}

	cmd.AddCommand(lightHeaderCmd(ctx))
	cmd.AddCommand(initLightCmd(ctx))
	cmd.AddCommand(updateLightCmd(ctx))
	cmd.AddCommand(deleteLightCmd(ctx))

	return cmd
}

func initLightCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:   "init [chain-id]",
		Short: "Initiate the light client",
		Long: `Initiate the light client by:
	1. passing it a root of trust as a --hash/-x and --height
	2. Use --force/-f to initialize from the configured node`,
		Args: cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			c, err := ctx.Config.GetChain(args[0])
			if err != nil {
				return err
			}
			chain := c.ChainI.(*tendermint.Chain)
			prover := c.ProverI.(*relay.Prover).GetUpstreamProver().(*tendermint.Prover)
			db, df, err := prover.NewLightDB()
			if err != nil {
				return err
			}
			defer df()

			force, err := cmd.Flags().GetBool(flagForce)
			if err != nil {
				return err
			}
			height, err := cmd.Flags().GetInt64(flags.FlagHeight)
			if err != nil {
				return err
			}
			hash, err := cmd.Flags().GetBytesHex(flagHash)
			if err != nil {
				return err
			}

			switch {
			case force: // force initialization from trusted node
				_, err := prover.LightClientWithoutTrust(db)
				if err != nil {
					return err
				}
				fmt.Printf("successfully created light client for %s by trusting endpoint %s...\n", chain.ChainID(), chain.Config().RpcAddr)
			case height > 0 && len(hash) > 0: // height and hash are given
				_, err = prover.LightClientWithTrust(db, prover.TrustOptions(height, hash))
				if err != nil {
					return wrapInitFailed(err)
				}
			default: // return error
				return errInitWrongFlags
			}

			return nil
		},
	}

	return forceFlag(lightFlags(cmd))
}

func updateLightCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:     "update [chain-id]",
		Aliases: []string{"u"},
		Short:   "Update the light client to latest header from configured node",
		Args:    cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			c, err := ctx.Config.GetChain(args[0])
			if err != nil {
				return err
			}
			prover := c.ProverI.(*relay.Prover).GetUpstreamProver().(*tendermint.Prover)
			bh, err := prover.GetLatestLightHeader()
			if err != nil {
				return err
			}

			ah, _, _, err := prover.UpdateLightWithHeader()
			if err != nil {
				return err
			}

			fmt.Printf("Updated light client for %s from height %d -> height %d\n", args[0], bh.Header.Height, ah.(*tmclient.Header).Header.Height)
			return nil
		},
	}

	return cmd
}

func lightHeaderCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:     "header [chain-id] [[height]]",
		Aliases: []string{"hdr"},
		Short:   "Get a header from the light client database",
		Long: "Get a header from the light client database. 0 returns last" +
			"trusted header and all others return the header at that height if stored",
		Args: cobra.RangeArgs(1, 2),
		RunE: func(cmd *cobra.Command, args []string) error {
			c, err := ctx.Config.GetChain(args[0])
			if err != nil {
				return err
			}
			chain := c.ChainI.(*tendermint.Chain)
			prover := c.ProverI.(*relay.Prover).GetUpstreamProver().(*tendermint.Prover)

			var header *tmclient.Header
			switch len(args) {
			case 1:
				header, err = prover.GetLatestLightHeader()
				if err != nil {
					return err
				}
			case 2:
				var height int64
				height, err = strconv.ParseInt(args[1], 10, 64) //convert to int64
				if err != nil {
					return err
				}

				if height == 0 {
					height, err = prover.GetLatestLightHeight()
					if err != nil {
						return err
					}

					if height == -1 {
						return tendermint.ErrLightNotInitialized
					}
				}

				header, err = prover.GetLightSignedHeaderAtHeight(height)
				if err != nil {
					return err
				}

			}

			out, err := chain.Codec().MarshalJSON(header)
			if err != nil {
				return err
			}

			fmt.Println(string(out))
			return nil
		},
	}
	return cmd
}

func deleteLightCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:     "delete [chain-id]",
		Aliases: []string{"d"},
		Short:   "wipe the light client database, forcing re-initialzation on the next run",
		Args:    cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			c, err := ctx.Config.GetChain(args[0])
			if err != nil {
				return err
			}
			prover := c.ProverI.(*relay.Prover).GetUpstreamProver().(*tendermint.Prover)
			err = prover.DeleteLightDB()
			if err != nil {
				return err
			}

			return nil
		},
	}
	return cmd
}
