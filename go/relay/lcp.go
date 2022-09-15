package relay

import (
	"context"
	"time"

	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/datachainlab/lcp/go/relay/elc"
	"github.com/datachainlab/lcp/go/relay/enclave"
	"github.com/datachainlab/lcp/go/sgx/ias"
	"github.com/hyperledger-labs/yui-relayer/core"
)

func registerEnclaveKey(pathEnd *core.PathEnd, prover *Prover, debug bool) error {
	if debug {
		ias.SetAllowDebugEnclaves()
		defer ias.UnsetAllowDebugEnclaves()
	}
	res, err := prover.client.AttestedVerificationReport(context.TODO(), &enclave.QueryAttestedVerificationReportRequest{})
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

func activateClient(pathEnd *core.PathEnd, srcProver *Prover, dst *core.ProvableChain) error {

	// 1. query the latest header from the upstream chain

	// TODO use latest or given height as option?
	header, err := srcProver.QueryLatestHeader() // origin prover's latest header
	if err != nil {
		return err
	}

	lcpQueryier := NewLCPClientQueryier(srcProver.client, srcProver.config.ElcClientId)
	header2, err := srcProver.originProver.SetupHeader(lcpQueryier, header)
	if err != nil {
		return err
	}
	anyHeader, err := clienttypes.PackHeader(header2)
	if err != nil {
		return err
	}

	// 2. send a request that contains a header from 1 to update the client in ELC

	res, err := srcProver.client.UpdateClient(context.TODO(), &elc.MsgUpdateClient{
		ClientId: srcProver.config.ElcClientId,
		Header:   anyHeader,
	})
	if err != nil {
		return err
	}

	// 3. make MsgUpdateClient with the result of 2 and send it to the downstream chain

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
