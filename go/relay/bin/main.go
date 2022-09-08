package main

import (
	"log"

	lcp "github.com/datachainlab/lcp/go/relay"
	"github.com/hyperledger-labs/yui-relayer/cmd"
)

func main() {
	if err := cmd.Execute(
		lcp.Module{},
	); err != nil {
		log.Fatal(err)
	}
}
