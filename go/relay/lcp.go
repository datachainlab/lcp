package relay

import (
	"context"
	"time"

	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
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
	signer, err := prover.upstreamChain.GetAddress()
	if err != nil {
		return err
	}
	msg, err := clienttypes.NewMsgUpdateClient(pathEnd.ClientID, header, signer.String())
	if err != nil {
		return err
	}
	if _, err := prover.upstreamChain.SendMsgs([]sdk.Msg{msg}); err != nil {
		return err
	}
	return nil
}
