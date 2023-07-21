package types

import (
	clienttypes "github.com/cosmos/ibc-go/v7/modules/core/02-client/types"
	"github.com/cosmos/ibc-go/v7/modules/core/exported"
)

var _ exported.ClientMessage = (*UpdateClientMessage)(nil)

func (UpdateClientMessage) ClientType() string {
	return ClientTypeLCP
}

func (m UpdateClientMessage) GetHeight() exported.Height {
	c, err := m.GetCommitment()
	if err != nil {
		panic(err)
	}
	return c.NewHeight
}

func (m UpdateClientMessage) ValidateBasic() error {
	if _, err := m.GetCommitment(); err != nil {
		return err
	}
	return nil
}

func (h UpdateClientMessage) GetCommitment() (*UpdateClientCommitment, error) {
	return ParseUpdateClientCommitment(h.Commitment)
}

var _ exported.ClientMessage = (*RegisterEnclaveKeyMessage)(nil)

func (RegisterEnclaveKeyMessage) ClientType() string {
	return ClientTypeLCP
}

func (RegisterEnclaveKeyMessage) GetHeight() exported.Height {
	// XXX: the header doesn't have height info, so return zero
	// this is just workaround until this function removed
	return clienttypes.ZeroHeight()
}

func (RegisterEnclaveKeyMessage) ValidateBasic() error {
	return nil
}
