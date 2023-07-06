package types

import (
	codectypes "github.com/cosmos/cosmos-sdk/codec/types"
	"github.com/cosmos/ibc-go/v7/modules/core/exported"
)

// RegisterInterfaces register the ibc channel submodule interfaces to protobuf
// Any.
func RegisterInterfaces(registry codectypes.InterfaceRegistry) {
	registry.RegisterImplementations(
		(*exported.ClientState)(nil),
		&ClientState{},
	)
	registry.RegisterImplementations(
		(*exported.ConsensusState)(nil),
		&ConsensusState{},
	)
	registry.RegisterImplementations(
		(*exported.ClientMessage)(nil),
		&UpdateClientHeader{},
		&RegisterEnclaveKeyHeader{},
	)
}
