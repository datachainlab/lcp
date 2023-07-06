package cmd

import (
	"encoding/json"
	"fmt"

	"github.com/cosmos/cosmos-sdk/crypto/hd"
	"github.com/hyperledger-labs/yui-relayer/chains/tendermint"
	"github.com/hyperledger-labs/yui-relayer/config"
	"github.com/spf13/cobra"
)

// keysCmd represents the keys command
func keysCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:     "keys",
		Aliases: []string{"k"},
		Short:   "manage keys held by the relayer for each chain",
	}

	cmd.AddCommand(
		keysAddCmd(ctx),
		keysRestoreCmd(ctx),
		keysShowCmd(ctx),
		keysListCmd(ctx),
	)

	return cmd
}

// keysAddCmd respresents the `keys add` command
func keysAddCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:     "add [chain-id] [[name]]",
		Aliases: []string{"a"},
		Short:   "adds a key to the keychain associated with a particular chain",
		Args:    cobra.RangeArgs(1, 2),
		RunE: func(cmd *cobra.Command, args []string) error {
			c, err := ctx.Config.GetChain(args[0])
			if err != nil {
				return err
			}
			chain := c.Chain.(*tendermint.Chain)

			var keyName string
			if len(args) == 2 {
				keyName = args[1]
			} else {
				keyName = chain.Key()
			}

			if chain.KeyExists(keyName) {
				return errKeyExists(keyName)
			}

			mnemonic, err := tendermint.CreateMnemonic()
			if err != nil {
				return err
			}

			info, err := chain.Keybase.NewAccount(keyName, mnemonic, "", hd.CreateHDPath(118, 0, 0).String(), hd.Secp256k1)
			if err != nil {
				return err
			}
			addr, err := info.GetAddress()
			if err != nil {
				return err
			}
			ko := keyOutput{Mnemonic: mnemonic, Address: addr.String()}

			out, err := json.Marshal(&ko)
			if err != nil {
				return err
			}

			fmt.Println(string(out))
			return nil
		},
	}

	return cmd
}

type keyOutput struct {
	Mnemonic string `json:"mnemonic" yaml:"mnemonic"`
	Address  string `json:"address" yaml:"address"`
}

// keysRestoreCmd respresents the `keys add` command
func keysRestoreCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:     "restore [chain-id] [name] [mnemonic]",
		Aliases: []string{"r"},
		Short:   "restores a mnemonic to the keychain associated with a particular chain",
		Args:    cobra.ExactArgs(3),
		RunE: func(cmd *cobra.Command, args []string) error {
			keyName := args[1]
			c, err := ctx.Config.GetChain(args[0])
			if err != nil {
				return err
			}
			chain := c.Chain.(*tendermint.Chain)

			if chain.KeyExists(keyName) {
				return errKeyExists(keyName)
			}

			info, err := chain.Keybase.NewAccount(keyName, args[2], "", hd.CreateHDPath(118, 0, 0).String(), hd.Secp256k1)
			if err != nil {
				return err
			}

			defer chain.UseSDKContext()()
			addr, err := info.GetAddress()
			if err != nil {
				return err
			}
			fmt.Println(addr.String())
			return nil
		},
	}

	return cmd
}

// keysShowCmd respresents the `keys show` command
func keysShowCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:     "show [chain-id] [[name]]",
		Aliases: []string{"s"},
		Short:   "shows a key from the keychain associated with a particular chain",
		Args:    cobra.RangeArgs(1, 2),
		RunE: func(cmd *cobra.Command, args []string) error {
			c, err := ctx.Config.GetChain(args[0])
			if err != nil {
				return err
			}
			chain := c.Chain.(*tendermint.Chain)

			var keyName string
			if len(args) == 2 {
				keyName = args[1]
			} else {
				keyName = chain.Key()
			}

			if !chain.KeyExists(keyName) {
				return errKeyDoesntExist(keyName)
			}

			info, err := chain.Keybase.Key(keyName)
			if err != nil {
				return err
			}

			addr, err := info.GetAddress()
			if err != nil {
				return err
			}
			fmt.Println(addr.String())
			return nil
		},
	}

	return cmd
}

// keysListCmd respresents the `keys list` command
func keysListCmd(ctx *config.Context) *cobra.Command {
	cmd := &cobra.Command{
		Use:   "list [chain-id]",
		Short: "lists keys from the keychain associated with a particular chain",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			c, err := ctx.Config.GetChain(args[0])
			if err != nil {
				return err
			}
			chain := c.Chain.(*tendermint.Chain)

			info, err := chain.Keybase.List()
			if err != nil {
				return err
			}

			for d, i := range info {
				addr, err := i.GetAddress()
				if err != nil {
					return err
				}
				fmt.Printf("key(%d): %s -> %s\n", d, i.Name, addr.String())
			}

			return nil
		},
	}

	return cmd
}
