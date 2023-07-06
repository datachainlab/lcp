package main

import (
	"os"

	"github.com/cosmos/cosmos-sdk/server"
	svrcmd "github.com/cosmos/cosmos-sdk/server/cmd"
	"github.com/datachainlab/lcp/go/sgx/ias"
	"github.com/datachainlab/lcp/go/simapp"
	"github.com/datachainlab/lcp/go/simapp/simd/cmd"
)

func main() {
	// WARNING: if you use the simd in production, you must remove the following code:
	ias.SetAllowDebugEnclaves()
	defer ias.UnsetAllowDebugEnclaves()

	rootCmd, _ := cmd.NewRootCmd()
	if err := svrcmd.Execute(rootCmd, "simd", simapp.DefaultNodeHome); err != nil {
		switch e := err.(type) {
		case server.ErrorCode:
			os.Exit(e.Code)
		default:
			os.Exit(1)
		}
	}
}
