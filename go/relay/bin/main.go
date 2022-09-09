package main

import (
	"log"

	lcp "github.com/datachainlab/lcp/go/relay"
	tendermint "github.com/hyperledger-labs/yui-relayer/chains/tendermint/module"
	"github.com/hyperledger-labs/yui-relayer/cmd"
)

func main() {
	if err := cmd.Execute(
		lcp.Module{},
		tendermint.Module{},
	); err != nil {
		log.Fatal(err)
	}
}
