package relay

import (
	"context"
	"log"
	"time"

	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	ibcexported "github.com/cosmos/ibc-go/v4/modules/core/exported"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/datachainlab/lcp/go/relay/elc"
	"github.com/datachainlab/lcp/go/relay/enclave"
	"github.com/datachainlab/lcp/go/sgx/ias"
	"github.com/hyperledger-labs/yui-relayer/core"
)

func (pr *Prover) syncUpstreamHeader(includeState bool) ([]*elc.MsgUpdateClientResponse, error) {

	// 1. check if the latest height of the client is less than the given height

	res, err := pr.lcpServiceClient.Client(context.TODO(), &elc.QueryClientRequest{ClientId: pr.config.ElcClientId})
	if err != nil {
		return nil, err
	}
	latestHeader, err := pr.originProver.GetLatestFinalizedHeader()
	if err != nil {
		return nil, err
	}

	var clientState ibcexported.ClientState
	if err := pr.codec.UnpackAny(res.ClientState, &clientState); err != nil {
		return nil, err
	}
	if clientState.GetLatestHeight().GTE(latestHeader.GetHeight()) {
		return nil, nil
	}

	log.Printf("syncUpstreamHeader try to update the client in ELC: latest=%v got=%v", clientState.GetLatestHeight(), latestHeader.GetHeight())

	// 2. query the header from the upstream chain

	headers, err := pr.originProver.SetupHeadersForUpdate(NewLCPQuerier(pr.lcpServiceClient, pr.config.ElcClientId), latestHeader)
	if err != nil {
		return nil, err
	}
	if len(headers) == 0 {
		return nil, nil
	}

	// 3. send a request that contains a header from 2 to update the client in ELC
	var responses []*elc.MsgUpdateClientResponse
	for _, header := range headers {
		anyHeader, err := clienttypes.PackHeader(header)
		if err != nil {
			return nil, err
		}
		res, err := pr.lcpServiceClient.UpdateClient(context.TODO(), &elc.MsgUpdateClient{
			ClientId:     pr.config.ElcClientId,
			Header:       anyHeader,
			IncludeState: includeState,
		})
		if err != nil {
			return nil, err
		}
		responses = append(responses, res)
	}

	return responses, nil
}

func registerEnclaveKey(pathEnd *core.PathEnd, prover *Prover, debug bool) error {
	if debug {
		ias.SetAllowDebugEnclaves()
		defer ias.UnsetAllowDebugEnclaves()
	}
	res, err := prover.lcpServiceClient.AttestedVerificationReport(context.TODO(), &enclave.QueryAttestedVerificationReportRequest{})
	if err != nil {
		return err
	}
	if err := ias.VerifyReport(res.Report, res.Signature, res.SigningCert, time.Now()); err != nil {
		return err
	}
	if _, err := ias.ParseAndValidateAVR(res.Report); err != nil {
		return err
	}
	header := &lcptypes.RegisterEnclaveKeyHeader{
		Report:      res.Report,
		Signature:   res.Signature,
		SigningCert: res.SigningCert,
	}
	signer, err := prover.originChain.GetAddress()
	if err != nil {
		return err
	}
	msg, err := clienttypes.NewMsgUpdateClient(pathEnd.ClientID, header, signer.String())
	if err != nil {
		return err
	}
	if _, err := prover.originChain.SendMsgs([]sdk.Msg{msg}); err != nil {
		return err
	}
	return nil
}

func activateClient(pathEnd *core.PathEnd, src, dst *core.ProvableChain) error {
	srcProver := src.Prover.(*Prover)
	if err := srcProver.initServiceClient(); err != nil {
		return err
	}

	// 1. LCP synchronises with the latest header of the upstream chain
	updates, err := srcProver.syncUpstreamHeader(true)
	if err != nil {
		return err
	}

	signer, err := dst.Chain.GetAddress()
	if err != nil {
		return err
	}

	// 2. Create a `MsgUpdateClient`s to apply to the LCP Client with the results of 1.
	var msgs []sdk.Msg
	for _, update := range updates {
		updateClientHeader := &lcptypes.UpdateClientHeader{
			Commitment: update.Commitment,
			Signer:     update.Signer,
			Signature:  update.Signature,
		}
		if err := updateClientHeader.ValidateBasic(); err != nil {
			return err
		}
		msg, err := clienttypes.NewMsgUpdateClient(pathEnd.ClientID, updateClientHeader, signer.String())
		if err != nil {
			return err
		}
		msgs = append(msgs, msg)
	}

	// 3. Submit the msgs to the LCP Client
	if _, err := dst.SendMsgs(msgs); err != nil {
		return err
	}
	return nil
}

type LCPQuerier struct {
	serviceClient LCPServiceClient
	clientID      string
}

var _ core.ChainInfoICS02Querier = (*LCPQuerier)(nil)

func NewLCPQuerier(serviceClient LCPServiceClient, clientID string) LCPQuerier {
	return LCPQuerier{
		serviceClient: serviceClient,
		clientID:      clientID,
	}
}

func (q LCPQuerier) ChainID() string {
	return "lcp"
}

// LatestHeight returns the latest height of the chain
func (q LCPQuerier) LatestHeight() (ibcexported.Height, error) {
	return clienttypes.ZeroHeight(), nil
}

// QueryClientState returns the client state of dst chain
// height represents the height of dst chain
func (q LCPQuerier) QueryClientState(ctx core.QueryContext) (*clienttypes.QueryClientStateResponse, error) {
	res, err := q.serviceClient.Client(ctx.Context(), &elc.QueryClientRequest{ClientId: q.clientID})
	if err != nil {
		return nil, err
	}
	return &clienttypes.QueryClientStateResponse{
		ClientState: res.ClientState,
	}, nil
}

// QueryClientConsensusState retrevies the latest consensus state for a client in state at a given height
func (q LCPQuerier) QueryClientConsensusState(ctx core.QueryContext, dstClientConsHeight ibcexported.Height) (*clienttypes.QueryConsensusStateResponse, error) {
	// TODO add query_client_consensus support to ecall-handler
	panic("not implemented error")
}
