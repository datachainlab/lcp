package relay

import (
	"context"
	"encoding/hex"
	"fmt"
	"log"
	"time"

	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	conntypes "github.com/cosmos/ibc-go/v4/modules/core/03-connection/types"
	chantypes "github.com/cosmos/ibc-go/v4/modules/core/04-channel/types"
	host "github.com/cosmos/ibc-go/v4/modules/core/24-host"
	ibcexported "github.com/cosmos/ibc-go/v4/modules/core/exported"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/datachainlab/lcp/go/relay/elc"
	"github.com/datachainlab/lcp/go/relay/ibc"
	"github.com/hyperledger-labs/yui-relayer/core"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type Prover struct {
	config       ProverConfig
	originChain  core.ChainI
	originProver core.ProverI

	codec            codec.ProtoCodecMarshaler
	path             *core.PathEnd
	lcpServiceClient LCPServiceClient
}

const (
	grpcTimeout = 30 * time.Second
)

var (
	_ core.ProverI = (*Prover)(nil)
)

func NewProver(config ProverConfig, originChain core.ChainI, originProver core.ProverI) (*Prover, error) {
	return &Prover{config: config, originChain: originChain, originProver: originProver}, nil
}

func (pr *Prover) GetOriginProver() core.ProverI {
	return pr.originProver
}

func (pr *Prover) initServiceClient() error {
	ctx, cancel := context.WithTimeout(context.Background(), grpcTimeout)
	defer cancel()
	conn, err := grpc.DialContext(
		ctx,
		pr.config.LcpServiceAddress,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		return err
	}
	pr.lcpServiceClient = NewLCPServiceClient(conn, pr.codec)
	return nil
}

// Init initializes the chain
func (pr *Prover) Init(homePath string, timeout time.Duration, codec codec.ProtoCodecMarshaler, debug bool) error {
	pr.codec = codec
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
	return pr.originChain.ChainID()
}

// QueryHeader returns the header corresponding to the height
func (pr *Prover) QueryHeader(height int64) (out core.HeaderI, err error) {
	return pr.originProver.QueryHeader(height)
}

// QueryLatestHeader returns the latest header from the chain
func (pr *Prover) QueryLatestHeader() (out core.HeaderI, err error) {
	return pr.originProver.QueryLatestHeader()
}

// GetLatestLightHeight returns the latest height on the light client
func (pr *Prover) GetLatestLightHeight() (int64, error) {
	return pr.originProver.GetLatestLightHeight()
}

// CreateMsgCreateClient creates a CreateClientMsg to this chain
func (pr *Prover) CreateMsgCreateClient(clientID string, dstHeader core.HeaderI, signer sdk.AccAddress) (*clienttypes.MsgCreateClient, error) {
	if err := pr.initServiceClient(); err != nil {
		return nil, err
	}
	msg, err := pr.originProver.CreateMsgCreateClient(clientID, dstHeader, signer)
	if err != nil {
		return nil, err
	}
	res, err := pr.lcpServiceClient.CreateClient(context.TODO(), &elc.MsgCreateClient{
		ClientState:    msg.ClientState,
		ConsensusState: msg.ConsensusState,
		Signer:         "", // TODO remove this field from the proto def
	})
	if err != nil {
		return nil, err
	}

	// TODO relayer should persist res.ClientId
	if pr.config.ElcClientId != res.ClientId {
		return nil, fmt.Errorf("you must specify '%v' as elc_client_id, but got %v", res.ClientId, pr.config.ElcClientId)
	}

	clientState := &lcptypes.ClientState{
		LatestHeight:         clienttypes.Height{},
		Mrenclave:            pr.config.GetMrenclave(),
		KeyExpiration:        60 * 60 * 24 * 7, // 7 days
		Keys:                 [][]byte{},
		AttestationTimes:     []uint64{},
		AllowedQuoteStatuses: pr.config.AllowedQuoteStatuses,
		AllowedAdvisoryIds:   pr.config.AllowedAdvisoryIds,
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

	resBytes, err := res.Marshal()
	if err != nil {
		return nil, err
	}
	log.Printf("ELC(%s) created. MsgCreateClientResponse: %s\n", res.ClientId, hex.EncodeToString(resBytes))

	// NOTE after creates client, register an enclave key into the client state
	return &clienttypes.MsgCreateClient{
		ClientState:    anyClientState,
		ConsensusState: anyConsensusState,
		Signer:         signer.String(),
	}, nil
}

// SetupHeader creates a new header based on a given header
func (pr *Prover) SetupHeader(dst core.LightClientIBCQueryierI, baseSrcHeader core.HeaderI) (core.HeaderI, error) {
	if err := pr.initServiceClient(); err != nil {
		return nil, err
	}

	baseSrcHeader, err := pr.originProver.SetupHeader(dst, baseSrcHeader)
	if err != nil {
		return nil, err
	}
	if baseSrcHeader == nil {
		return nil, nil
	}
	anyHeader, err := clienttypes.PackHeader(baseSrcHeader)
	if err != nil {
		return nil, err
	}
	msg := elc.MsgUpdateClient{
		ClientId: pr.config.ElcClientId,
		Header:   anyHeader,
	}
	res, err := pr.lcpServiceClient.UpdateClient(context.TODO(), &msg)
	if err != nil {
		return nil, err
	}
	if _, err := lcptypes.ParseUpdateClientCommitment(res.Commitment); err != nil {
		return nil, err
	}
	return &lcptypes.UpdateClientHeader{
		Commitment: res.Commitment,
		Signer:     res.Signer,
		Signature:  res.Signature,
	}, nil
}

// UpdateLightWithHeader updates a header on the light client and returns the header and height corresponding to the chain
func (pr *Prover) UpdateLightWithHeader() (header core.HeaderI, provableHeight int64, queryableHeight int64, err error) {
	return pr.originProver.UpdateLightWithHeader()
}

// QueryClientConsensusState returns the ClientConsensusState and its proof
func (pr *Prover) QueryClientConsensusStateWithProof(height int64, dstClientConsHeight ibcexported.Height) (*clienttypes.QueryConsensusStateResponse, error) {
	res, err := pr.originProver.QueryClientConsensusStateWithProof(height, dstClientConsHeight)
	if err != nil {
		return nil, err
	}
	res2, err := pr.lcpServiceClient.VerifyClientConsensus(
		context.TODO(),
		&ibc.MsgVerifyClientConsensus{
			ClientId:                        pr.config.ElcClientId,
			Prefix:                          []byte(host.StoreKey),
			CounterpartyClientId:            pr.path.ClientID,
			ConsensusHeight:                 dstClientConsHeight.(clienttypes.Height),
			ExpectedAnyClientConsensusState: res.ConsensusState,
			ProofHeight:                     res.ProofHeight,
			Proof:                           res.Proof,
		},
	)
	if err != nil {
		return nil, err
	}
	commitment, err := lcptypes.ParseStateCommitment(res2.Commitment)
	if err != nil {
		return nil, err
	}
	return &clienttypes.QueryConsensusStateResponse{
		ConsensusState: res.ConsensusState,
		Proof:          lcptypes.NewStateCommitmentProof(res2.Commitment, res2.Signer, res2.Signature).ToRLPBytes(),
		ProofHeight:    commitment.Height,
	}, nil
}

// QueryClientStateWithProof returns the ClientState and its proof
func (pr *Prover) QueryClientStateWithProof(height int64) (*clienttypes.QueryClientStateResponse, error) {
	res, err := pr.originProver.QueryClientStateWithProof(height)
	if err != nil {
		return nil, err
	}

	res2, err := pr.lcpServiceClient.VerifyClient(
		context.TODO(),
		&ibc.MsgVerifyClient{
			ClientId:               pr.config.ElcClientId,
			Prefix:                 []byte(host.StoreKey),
			CounterpartyClientId:   pr.path.ClientID,
			ExpectedAnyClientState: res.ClientState,
			ProofHeight:            res.ProofHeight,
			Proof:                  res.Proof,
		},
	)
	if err != nil {
		return nil, err
	}

	commitment, err := lcptypes.ParseStateCommitment(res2.Commitment)
	if err != nil {
		return nil, err
	}
	return &clienttypes.QueryClientStateResponse{
		ClientState: res.ClientState,
		Proof:       lcptypes.NewStateCommitmentProof(res2.Commitment, res2.Signer, res2.Signature).ToRLPBytes(),
		ProofHeight: commitment.Height,
	}, nil
}

// QueryConnectionWithProof returns the Connection and its proof
func (pr *Prover) QueryConnectionWithProof(height int64) (*conntypes.QueryConnectionResponse, error) {
	res, err := pr.originProver.QueryConnectionWithProof(height)
	if err != nil {
		return nil, err
	}
	// NOTE: if res.Proof length is zero, this means that the connection doesn't exist
	if len(res.Proof) == 0 {
		return res, nil
	}

	res2, err := pr.lcpServiceClient.VerifyConnection(
		context.TODO(),
		&ibc.MsgVerifyConnection{
			ClientId:           pr.config.ElcClientId,
			Prefix:             []byte(host.StoreKey),
			ConnectionId:       pr.path.ConnectionID,
			ExpectedConnection: *res.Connection,
			ProofHeight:        res.ProofHeight,
			Proof:              res.Proof,
		},
	)
	if err != nil {
		return nil, err
	}

	commitment, err := lcptypes.ParseStateCommitment(res2.Commitment)
	if err != nil {
		return nil, err
	}
	return &conntypes.QueryConnectionResponse{
		Connection:  res.Connection,
		Proof:       lcptypes.NewStateCommitmentProof(res2.Commitment, res2.Signer, res2.Signature).ToRLPBytes(),
		ProofHeight: commitment.Height,
	}, nil
}

// QueryChannelWithProof returns the Channel and its proof
func (pr *Prover) QueryChannelWithProof(height int64) (chanRes *chantypes.QueryChannelResponse, err error) {
	res, err := pr.originProver.QueryChannelWithProof(height)
	if err != nil {
		return nil, err
	}
	// NOTE: if res.Proof length is zero, this means that the connection doesn't exist
	if len(res.Proof) == 0 {
		return res, nil
	}

	res2, err := pr.lcpServiceClient.VerifyChannel(
		context.TODO(),
		&ibc.MsgVerifyChannel{
			ClientId:        pr.config.ElcClientId,
			Prefix:          []byte(host.StoreKey),
			PortId:          pr.path.PortID,
			ChannelId:       pr.path.ChannelID,
			ExpectedChannel: *res.Channel,
			ProofHeight:     res.ProofHeight,
			Proof:           res.Proof,
		},
	)
	if err != nil {
		return nil, err
	}

	commitment, err := lcptypes.ParseStateCommitment(res2.Commitment)
	if err != nil {
		return nil, err
	}
	return &chantypes.QueryChannelResponse{
		Channel:     res.Channel,
		Proof:       lcptypes.NewStateCommitmentProof(res2.Commitment, res2.Signer, res2.Signature).ToRLPBytes(),
		ProofHeight: commitment.Height,
	}, nil
}

// QueryPacketCommitmentWithProof returns the packet commitment and its proof
func (pr *Prover) QueryPacketCommitmentWithProof(height int64, seq uint64) (comRes *chantypes.QueryPacketCommitmentResponse, err error) {
	if err := pr.initServiceClient(); err != nil {
		return nil, err
	}
	if _, err := pr.syncUpstreamHeader(height, false); err != nil {
		return nil, err
	}

	res, err := pr.originProver.QueryPacketCommitmentWithProof(height, seq)
	if err != nil {
		return nil, err
	}

	res2, err := pr.lcpServiceClient.VerifyPacket(context.TODO(), &ibc.MsgVerifyPacket{
		ClientId:    pr.config.ElcClientId,
		Prefix:      []byte(host.StoreKey),
		PortId:      pr.path.PortID,
		ChannelId:   pr.path.ChannelID,
		Sequence:    seq,
		Commitment:  res.Commitment,
		ProofHeight: res.ProofHeight,
		Proof:       res.Proof,
	})
	if err != nil {
		return nil, err
	}
	commitment, err := lcptypes.ParseStateCommitment(res2.Commitment)
	if err != nil {
		return nil, err
	}
	return &chantypes.QueryPacketCommitmentResponse{
		Commitment:  res.Commitment,
		Proof:       lcptypes.NewStateCommitmentProof(res2.Commitment, res2.Signer, res2.Signature).ToRLPBytes(),
		ProofHeight: commitment.Height,
	}, nil
}

// QueryPacketAcknowledgementCommitmentWithProof returns the packet acknowledgement commitment and its proof
func (pr *Prover) QueryPacketAcknowledgementCommitmentWithProof(height int64, seq uint64) (ackRes *chantypes.QueryPacketAcknowledgementResponse, err error) {
	if err := pr.initServiceClient(); err != nil {
		return nil, err
	}
	if _, err := pr.syncUpstreamHeader(height, false); err != nil {
		return nil, err
	}

	res, err := pr.originProver.QueryPacketAcknowledgementCommitmentWithProof(height, seq)
	if err != nil {
		return nil, err
	}
	res2, err := pr.lcpServiceClient.VerifyPacketAcknowledgement(
		context.TODO(),
		&ibc.MsgVerifyPacketAcknowledgement{
			ClientId:    pr.config.ElcClientId,
			Prefix:      []byte(host.StoreKey),
			PortId:      pr.path.PortID,
			ChannelId:   pr.path.ChannelID,
			Sequence:    seq,
			Commitment:  res.Acknowledgement,
			ProofHeight: res.ProofHeight,
			Proof:       res.Proof,
		},
	)
	if err != nil {
		return nil, err
	}
	commitment, err := lcptypes.ParseStateCommitment(res2.Commitment)
	if err != nil {
		return nil, err
	}
	return &chantypes.QueryPacketAcknowledgementResponse{
		Acknowledgement: res.Acknowledgement,
		Proof:           lcptypes.NewStateCommitmentProof(res2.Commitment, res2.Signer, res2.Signature).ToRLPBytes(),
		ProofHeight:     commitment.Height,
	}, err
}
