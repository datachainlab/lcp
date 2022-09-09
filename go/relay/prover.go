package relay

import (
	"context"
	"log"
	"time"

	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	conntypes "github.com/cosmos/ibc-go/v4/modules/core/03-connection/types"
	chantypes "github.com/cosmos/ibc-go/v4/modules/core/04-channel/types"
	ibcexported "github.com/cosmos/ibc-go/v4/modules/core/exported"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/datachainlab/lcp/go/relay/elc"
	"github.com/hyperledger-labs/yui-relayer/core"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type Prover struct {
	config         ProverConfig
	upstreamChain  core.ChainI
	upstreamProver core.ProverI

	path   *core.PathEnd
	client LCPServiceClient
}

var (
	_ core.ProverI = (*Prover)(nil)
)

func NewProver(config ProverConfig, upstreamChain core.ChainI, upstreamProver core.ProverI) (*Prover, error) {
	return &Prover{config: config, upstreamChain: upstreamChain, upstreamProver: upstreamProver}, nil
}

func (pr *Prover) GetUpstreamProver() core.ProverI {
	return pr.upstreamProver
}

func (pr *Prover) initServiceClient() error {
	conn, err := grpc.Dial(
		pr.config.LcpServiceAddress,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		return err
	}
	pr.client = NewLCPServiceClient(conn)
	return nil
}

// Init initializes the chain
func (pr *Prover) Init(homePath string, timeout time.Duration, codec codec.ProtoCodecMarshaler, debug bool) error {
	return nil
}

// SetRelayInfo sets source's path and counterparty's info to the chain
func (pr *Prover) SetRelayInfo(path *core.PathEnd, counterparty *core.ProvableChain, counterpartyPath *core.PathEnd) error {
	pr.path = path
	return nil
}

// SetupForRelay performs chain-specific setup before starting the relay
func (pr *Prover) SetupForRelay(ctx context.Context) error {
	return pr.initServiceClient()
}

// GetChainID returns the chain ID
func (pr *Prover) GetChainID() string {
	return pr.upstreamChain.ChainID()
}

// QueryLatestHeader returns the latest header from the chain
func (pr *Prover) QueryLatestHeader() (out core.HeaderI, err error) {
	return pr.upstreamProver.QueryLatestHeader()
}

// GetLatestLightHeight returns the latest height on the light client
func (pr *Prover) GetLatestLightHeight() (int64, error) {
	panic("not implemented") // TODO: Implement
}

// CreateMsgCreateClient creates a CreateClientMsg to this chain
func (pr *Prover) CreateMsgCreateClient(clientID string, dstHeader core.HeaderI, signer sdk.AccAddress) (*clienttypes.MsgCreateClient, error) {
	if err := pr.initServiceClient(); err != nil {
		return nil, err
	}
	msg, err := pr.upstreamProver.CreateMsgCreateClient(clientID, dstHeader, signer)
	if err != nil {
		return nil, err
	}
	res, err := pr.client.CreateClient(context.TODO(), &elc.MsgCreateClient{
		ClientState:    msg.ClientState,
		ConsensusState: msg.ConsensusState,
		Signer:         "", // TODO remove this field from the proto def
	})
	if err != nil {
		return nil, err
	}
	// TODO relayer should persist res.ClientId
	log.Println("upstreamClientID:", res.ClientId)

	clientState := &lcptypes.ClientState{
		LatestHeight:  clienttypes.Height{},
		Mrenclave:     pr.config.GetMrenclave(),
		KeyExpiration: 0,
		Keys:          [][]byte{},
	}
	consensusState := &lcptypes.ConsensusState{}

	anyClientState, err := clienttypes.PackClientState(clientState)
	if err != nil {
		return nil, err
	}
	anyConsensusState, err := clienttypes.PackConsensusState(consensusState)
	if err != nil {
		return nil, err
	}

	// NOTE after creates client, register an enclave key into the client state
	return &clienttypes.MsgCreateClient{
		ClientState:    anyClientState,
		ConsensusState: anyConsensusState,
		Signer:         signer.String(),
	}, nil
}

// SetupHeader creates a new header based on a given header
func (pr *Prover) SetupHeader(dst core.LightClientIBCQueryierI, baseSrcHeader core.HeaderI) (core.HeaderI, error) {
	panic("not implemented") // TODO: Implement
}

// UpdateLightWithHeader updates a header on the light client and returns the header and height corresponding to the chain
func (pr *Prover) UpdateLightWithHeader() (header core.HeaderI, provableHeight int64, queryableHeight int64, err error) {
	panic("not implemented") // TODO: Implement
}

// QueryClientConsensusState returns the ClientConsensusState and its proof
func (pr *Prover) QueryClientConsensusStateWithProof(height int64, dstClientConsHeight ibcexported.Height) (*clienttypes.QueryConsensusStateResponse, error) {
	panic("not implemented") // TODO: Implement
}

// QueryClientStateWithProof returns the ClientState and its proof
func (pr *Prover) QueryClientStateWithProof(height int64) (*clienttypes.QueryClientStateResponse, error) {
	panic("not implemented") // TODO: Implement
}

// QueryConnectionWithProof returns the Connection and its proof
func (pr *Prover) QueryConnectionWithProof(height int64) (*conntypes.QueryConnectionResponse, error) {
	panic("not implemented") // TODO: Implement
}

// QueryChannelWithProof returns the Channel and its proof
func (pr *Prover) QueryChannelWithProof(height int64) (chanRes *chantypes.QueryChannelResponse, err error) {
	panic("not implemented") // TODO: Implement
}

// QueryPacketCommitmentWithProof returns the packet commitment and its proof
func (pr *Prover) QueryPacketCommitmentWithProof(height int64, seq uint64) (comRes *chantypes.QueryPacketCommitmentResponse, err error) {
	panic("not implemented") // TODO: Implement
}

// QueryPacketAcknowledgementCommitmentWithProof returns the packet acknowledgement commitment and its proof
func (pr *Prover) QueryPacketAcknowledgementCommitmentWithProof(height int64, seq uint64) (ackRes *chantypes.QueryPacketAcknowledgementResponse, err error) {
	panic("not implemented") // TODO: Implement
}
