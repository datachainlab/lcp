package types

import (
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	"github.com/cosmos/ibc-go/v4/modules/core/exported"
)

var _ exported.Header = (*UpdateClientHeader)(nil)

func (h UpdateClientHeader) ClientType() string {
	return ClientTypeLCP
}

// NOTE: `GetHeight` will be removed in the near feature
func (h UpdateClientHeader) GetHeight() exported.Height {
	c, err := h.GetCommitment()
	if err != nil {
		panic(err)
	}
	return c.NewHeight
}

func (h UpdateClientHeader) ValidateBasic() error {
	if _, err := h.GetCommitment(); err != nil {
		return err
	}
	return nil
}

func (h UpdateClientHeader) GetCommitment() (*UpdateClientCommitment, error) {
	return ParseUpdateClientCommitment(h.Commitment)
}

var _ exported.Header = (*RegisterEnclaveKeyHeader)(nil)

func (h RegisterEnclaveKeyHeader) ClientType() string {
	return ClientTypeLCP
}

// NOTE: `GetHeight` will be removed in the near feature
func (h RegisterEnclaveKeyHeader) GetHeight() exported.Height {
	// XXX: the header doesn't have height info, so return zero
	// this is just workaround until this function removed
	return clienttypes.ZeroHeight()
}

func (h RegisterEnclaveKeyHeader) ValidateBasic() error {
	return nil
}
