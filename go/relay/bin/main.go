package main

import (
	"log"

	lcp "github.com/datachainlab/lcp/go/relay"
	lcptm "github.com/datachainlab/lcp/go/relay/tendermint"
	tendermint "github.com/hyperledger-labs/yui-relayer/chains/tendermint/module"
	"github.com/hyperledger-labs/yui-relayer/cmd"
)

func main() {
	if err := cmd.Execute(
		tendermint.Module{},
		lcp.Module{},
		lcptm.Module{},
	); err != nil {
		log.Fatal(err)
	}
}
