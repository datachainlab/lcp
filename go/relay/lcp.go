package relay

import (
	"context"
	"log"
	"time"

	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	"github.com/cosmos/ibc-go/v4/modules/core/exported"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/datachainlab/lcp/go/relay/elc"
	"github.com/datachainlab/lcp/go/relay/enclave"
	"github.com/datachainlab/lcp/go/sgx/ias"
	"github.com/hyperledger-labs/yui-relayer/core"
)

func (pr *Prover) syncUpstreamHeader(height int64) (*elc.MsgUpdateClientResponse, error) {

	// 1. check if the latest height of the client is less than the given height

	res, err := pr.lcpServiceClient.Client(context.TODO(), &elc.QueryClientRequest{ClientId: pr.config.ElcClientId})
	if err != nil {
		return nil, err
	}
	var clientState exported.ClientState
	if err := pr.codec.UnpackAny(res.ClientState, &clientState); err != nil {
		return nil, err
	}
	if clientState.GetLatestHeight().GetRevisionHeight() >= uint64(height) {
		return nil, nil
	}

	log.Printf("syncUpstreamHeader try to update the client in ELC: latest=%v got=%v", clientState.GetLatestHeight().GetRevisionHeight(), height)

	// 2. query the header from the upstream chain

	lcpQuerier := NewLCPClientQueryier(pr.lcpServiceClient, pr.config.ElcClientId)
	h, err := pr.originProver.QueryHeader(height)
	if err != nil {
		return nil, err
	}
	header, err := pr.originProver.SetupHeader(lcpQuerier, h)
	if err != nil {
		return nil, err
	}
	anyHeader, err := clienttypes.PackHeader(header)
	if err != nil {
		return nil, err
	}

	// 3. send a request that contains a header from 2 to update the client in ELC

	return pr.lcpServiceClient.UpdateClient(context.TODO(), &elc.MsgUpdateClient{
		ClientId: pr.config.ElcClientId,
		Header:   anyHeader,
	})
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
	latestHeight, err := src.GetLatestHeight()
	if err != nil {
		return err
	}
	srcProver := src.ProverI.(*Prover)
	if err := srcProver.initServiceClient(); err != nil {
		return err
	}

	// LCP synchronizes with the latest header of the upstream chain

	res, err := srcProver.syncUpstreamHeader(latestHeight)
	if err != nil {
		return err
	}

	// make MsgUpdateClient with the result of 2 and send it to the downstream chain

	updateClientHeader := &lcptypes.UpdateClientHeader{
		Commitment: res.Commitment,
		Signer:     res.Signer,
		Signature:  res.Signature,
	}
	if err := updateClientHeader.ValidateBasic(); err != nil {
		return err
	}
	signer, err := dst.ChainI.GetAddress()
	if err != nil {
		return err
	}
	msg, err := clienttypes.NewMsgUpdateClient(pathEnd.ClientID, updateClientHeader, signer.String())
	if err != nil {
		return err
	}
	if _, err := dst.SendMsgs([]sdk.Msg{msg}); err != nil {
		return err
	}
	return nil
}

type LCPClientQueryier struct {
	serviceClient LCPServiceClient
	clientID      string

	core.LightClientIBCQueryierI
}

func NewLCPClientQueryier(serviceClient LCPServiceClient, clientID string) LCPClientQueryier {
	return LCPClientQueryier{
		serviceClient: serviceClient,
		clientID:      clientID,
	}
}

func (q LCPClientQueryier) GetLatestLightHeight() (int64, error) {
	return 0, nil
}

func (q LCPClientQueryier) QueryClientState(_ int64) (*clienttypes.QueryClientStateResponse, error) {
	res, err := q.serviceClient.Client(context.TODO(), &elc.QueryClientRequest{ClientId: q.clientID})
	if err != nil {
		return nil, err
	}
	return &clienttypes.QueryClientStateResponse{
		ClientState: res.ClientState,
	}, nil
}
