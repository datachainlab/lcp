package types

import "github.com/cosmos/ibc-go/v4/modules/core/exported"

var _ exported.ConsensusState = (*ConsensusState)(nil)

func (cs ConsensusState) ClientType() string {
	return ClientTypeLCP
}

// GetRoot returns the commitment root of the consensus state,
// which is used for key-value pair verification.
func (cs ConsensusState) GetRoot() exported.Root {
	panic("not implemented") // TODO: Implement
}

// GetTimestamp returns the timestamp (in nanoseconds) of the consensus state
func (cs ConsensusState) GetTimestamp() uint64 {
	return cs.Timestamp
}

func (cs ConsensusState) ValidateBasic() error {
	return nil
}
