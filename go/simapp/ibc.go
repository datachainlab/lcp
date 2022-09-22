package simapp

import (
	"fmt"

	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	paramtypes "github.com/cosmos/cosmos-sdk/x/params/types"
	clientkeeper "github.com/cosmos/ibc-go/v4/modules/core/02-client/keeper"
	connectionkeeper "github.com/cosmos/ibc-go/v4/modules/core/03-connection/keeper"
	connectiontypes "github.com/cosmos/ibc-go/v4/modules/core/03-connection/types"
	channeltypes "github.com/cosmos/ibc-go/v4/modules/core/04-channel/types"
	"github.com/cosmos/ibc-go/v4/modules/core/exported"
	ibckeeper "github.com/cosmos/ibc-go/v4/modules/core/keeper"
	tenderminttypes "github.com/cosmos/ibc-go/v4/modules/light-clients/07-tendermint/types"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
)

func overrideIBCClientKeeper(k ibckeeper.Keeper, cdc codec.BinaryCodec, key sdk.StoreKey, paramSpace paramtypes.Subspace) *ibckeeper.Keeper {
	clientKeeper := NewClientKeeper(k.ClientKeeper)
	k.ConnectionKeeper = connectionkeeper.NewKeeper(cdc, key, paramSpace, clientKeeper)
	return &k
}

var _ connectiontypes.ClientKeeper = (*ClientKeeper)(nil)
var _ channeltypes.ClientKeeper = (*ClientKeeper)(nil)

// ClientKeeper override `GetSelfConsensusState` and `ValidateSelfClient` in the keeper of ibc-client
// original method doesn't yet support a consensus state for general client
type ClientKeeper struct {
	clientkeeper.Keeper
}

func NewClientKeeper(k clientkeeper.Keeper) ClientKeeper {
	return ClientKeeper{Keeper: k}
}

func (k ClientKeeper) ValidateSelfClient(ctx sdk.Context, clientState exported.ClientState) error {
	switch cs := clientState.(type) {
	case *tenderminttypes.ClientState:
		return k.Keeper.ValidateSelfClient(ctx, cs)
	case *lcptypes.ClientState:
		return nil
	// case *mocktypes.ClientState:
	// return nil
	default:
		return fmt.Errorf("unexpected client state type: %T", cs)
	}
}
