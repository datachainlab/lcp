package relay

import (
	"context"
	"fmt"
	"time"

	"github.com/cosmos/cosmos-sdk/codec"
	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v7/modules/core/02-client/types"
	"github.com/cosmos/ibc-go/v7/modules/core/exported"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/datachainlab/lcp/go/relay/elc"
	"github.com/datachainlab/lcp/go/relay/enclave"
	"github.com/hyperledger-labs/yui-relayer/core"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

type Prover struct {
	config       ProverConfig
	originChain  core.Chain
	originProver core.Prover

	homePath string
	codec    codec.ProtoCodecMarshaler
	path     *core.PathEnd

	lcpServiceClient LCPServiceClient
	activeEnclaveKey *enclave.EnclaveKeyInfo
}

var (
	_ core.Prover = (*Prover)(nil)
)

func NewProver(config ProverConfig, originChain core.Chain, originProver core.Prover) (*Prover, error) {
	return &Prover{config: config, originChain: originChain, originProver: originProver}, nil
}

func (pr *Prover) GetOriginProver() core.Prover {
	return pr.originProver
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
	pr.lcpServiceClient = NewLCPServiceClient(conn, pr.codec)
	return nil
}

// Init initializes the chain
func (pr *Prover) Init(homePath string, timeout time.Duration, codec codec.ProtoCodecMarshaler, debug bool) error {
	if err := pr.originChain.Init(homePath, timeout, codec, debug); err != nil {
		return err
	}
	if err := pr.originProver.Init(homePath, timeout, codec, debug); err != nil {
		return err
	}
	pr.homePath = homePath
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

// CreateMsgCreateClient creates a CreateClientMsg to this chain
func (pr *Prover) CreateMsgCreateClient(clientID string, dstHeader core.Header, signer sdk.AccAddress) (*clienttypes.MsgCreateClient, error) {
	if err := pr.initServiceClient(); err != nil {
		return nil, err
	}
	// NOTE: Query the LCP for available keys, but no need to register it into on-chain here
	keysRes, err := pr.lcpServiceClient.AvailableEnclaveKeys(context.TODO(), &enclave.QueryAvailableEnclaveKeysRequest{})
	if err != nil {
		return nil, err
	} else if len(keysRes.Keys) == 0 {
		return nil, fmt.Errorf("no available enclave keys")
	}
	msg, err := pr.originProver.CreateMsgCreateClient(clientID, dstHeader, signer)
	if err != nil {
		return nil, err
	}
	res, err := pr.lcpServiceClient.CreateClient(context.TODO(), &elc.MsgCreateClient{
		ClientState:    msg.ClientState,
		ConsensusState: msg.ConsensusState,
		Signer:         keysRes.Keys[0].EnclaveKeyAddress,
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
		KeyExpiration:        defaultEnclaveKeyExpiration, // TODO configurable
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

	// NOTE after creates client, register an enclave key into the client state
	return &clienttypes.MsgCreateClient{
		ClientState:    anyClientState,
		ConsensusState: anyConsensusState,
		Signer:         signer.String(),
	}, nil
}

// GetLatestFinalizedHeader returns the latest finalized header on this chain
// The returned header is expected to be the latest one of headers that can be verified by the light client
func (pr *Prover) GetLatestFinalizedHeader() (latestFinalizedHeader core.Header, err error) {
	return pr.originProver.GetLatestFinalizedHeader()
}

// SetupHeadersForUpdate returns the finalized header and any intermediate headers needed to apply it to the client on the counterpaty chain
// The order of the returned header slice should be as: [<intermediate headers>..., <update header>]
// if the header slice's length == nil and err == nil, the relayer should skips the update-client
func (pr *Prover) SetupHeadersForUpdate(dstChain core.ChainInfoICS02Querier, latestFinalizedHeader core.Header) ([]core.Header, error) {
	if err := pr.ensureAvailableEnclaveKeyExists(context.TODO()); err != nil {
		return nil, err
	}

	headers, err := pr.originProver.SetupHeadersForUpdate(dstChain, latestFinalizedHeader)
	if err != nil {
		return nil, err
	}
	if len(headers) == 0 {
		return nil, nil
	}
	var updates []core.Header
	for _, h := range headers {
		anyHeader, err := clienttypes.PackClientMessage(h)
		if err != nil {
			return nil, err
		}
		res, err := pr.lcpServiceClient.UpdateClient(context.TODO(), &elc.MsgUpdateClient{
			ClientId:     pr.config.ElcClientId,
			Header:       anyHeader,
			IncludeState: false,
			Signer:       pr.activeEnclaveKey.EnclaveKeyAddress,
		})
		if err != nil {
			return nil, err
		}
		if _, err := lcptypes.ParseUpdateClientCommitment(res.Commitment); err != nil {
			return nil, err
		}
		updates = append(updates, &lcptypes.UpdateClientHeader{
			Commitment: res.Commitment,
			Signer:     res.Signer,
			Signature:  res.Signature,
		})
	}
	return updates, nil
}

func (pr *Prover) ProveState(ctx core.QueryContext, path string, value []byte) ([]byte, clienttypes.Height, error) {
	proof, proofHeight, err := pr.originProver.ProveState(ctx, path, value)
	if err != nil {
		return nil, clienttypes.Height{}, err
	}
	res, err := pr.lcpServiceClient.VerifyMembership(ctx.Context(), &elc.MsgVerifyMembership{
		ClientId:    pr.config.ElcClientId,
		Prefix:      []byte(exported.StoreKey),
		Path:        path,
		Value:       value,
		ProofHeight: proofHeight,
		Proof:       proof,
		Signer:      pr.activeEnclaveKey.EnclaveKeyAddress,
	})
	if err != nil {
		return nil, clienttypes.Height{}, err
	}
	commitment, err := lcptypes.ParseStateCommitment(res.Commitment)
	if err != nil {
		return nil, clienttypes.Height{}, err
	}
	return lcptypes.NewStateCommitmentProof(res.Commitment, res.Signer, res.Signature).ToRLPBytes(), commitment.Height, nil
}
