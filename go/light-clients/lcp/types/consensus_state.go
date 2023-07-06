package types

import "github.com/cosmos/ibc-go/v7/modules/core/exported"

var _ exported.ConsensusState = (*ConsensusState)(nil)

func (cs ConsensusState) ClientType() string {
	return ClientTypeLCP
}

// GetTimestamp returns the timestamp (in nanoseconds) of the consensus state
func (cs ConsensusState) GetTimestamp() uint64 {
	return cs.Timestamp
}

func (cs ConsensusState) ValidateBasic() error {
	return nil
}
