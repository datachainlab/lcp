package relay

import (
	"context"

	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v4/modules/core/02-client/types"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/datachainlab/lcp/go/relay/enclave"
)

func registerEnclaveKey(prover *Prover) error {
	res, err := prover.client.AttestedVerificationReport(context.TODO(), &enclave.QueryAttestedVerificationReportRequest{})
	if err != nil {
		return err
	}
	header := &lcptypes.RegisterEnclaveKeyHeader{
		Report:    res.Report,
		Signature: res.Signature,
	}
	signer, err := prover.upstreamChain.GetAddress()
	if err != nil {
		return err
	}
	msg, err := clienttypes.NewMsgUpdateClient(prover.config.UpstreamClientId, header, signer.String())
	if err != nil {
		return err
	}
	if _, err := prover.upstreamChain.SendMsgs([]sdk.Msg{msg}); err != nil {
		return err
	}
	return nil
}
