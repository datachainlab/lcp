package relay

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"time"

	sdk "github.com/cosmos/cosmos-sdk/types"
	clienttypes "github.com/cosmos/ibc-go/v7/modules/core/02-client/types"
	ibcexported "github.com/cosmos/ibc-go/v7/modules/core/exported"
	lcptypes "github.com/datachainlab/lcp/go/light-clients/lcp/types"
	"github.com/datachainlab/lcp/go/relay/elc"
	"github.com/datachainlab/lcp/go/relay/enclave"
	"github.com/datachainlab/lcp/go/sgx/ias"
	mapset "github.com/deckarep/golang-set/v2"
	"github.com/hyperledger-labs/yui-relayer/core"
	oias "github.com/oasisprotocol/oasis-core/go/common/sgx/ias"
)

const lastEnclaveKeyInfoFile = "last_eki"

var ErrLastEnclaveKeyInfoNotFound = errors.New("last enclave key info not found")

func (pr *Prover) loadLastEnclaveKey(ctx context.Context) (*enclave.EnclaveKeyInfo, error) {
	path, err := pr.lastEnclaveKeyInfoFilePath()
	if err != nil {
		return nil, err
	}
	bz, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, fmt.Errorf("%v not found: %w", path, ErrLastEnclaveKeyInfoNotFound)
		}
		return nil, err
	}
	var eki enclave.EnclaveKeyInfo
	if err := json.Unmarshal(bz, &eki); err != nil {
		return nil, err
	}
	return &eki, nil
}

func (pr *Prover) saveLastEnclaveKey(ctx context.Context, eki *enclave.EnclaveKeyInfo) error {
	path, err := pr.lastEnclaveKeyInfoFilePath()
	if err != nil {
		return err
	}
	f, err := os.CreateTemp(os.TempDir(), lastEnclaveKeyInfoFile)
	if err != nil {
		return err
	}
	defer f.Close()

	bz, err := json.Marshal(eki)
	if err != nil {
		return err
	}
	if _, err := f.Write(bz); err != nil {
		return err
	}
	if err := os.Rename(f.Name(), path); err != nil {
		return err
	}
	return nil
}

func (pr *Prover) lastEnclaveKeyInfoFilePath() (string, error) {
	path := filepath.Join(pr.homePath, "lcp", pr.originChain.ChainID())
	if err := os.MkdirAll(path, os.ModePerm); err != nil {
		return "", err
	}
	return filepath.Join(path, lastEnclaveKeyInfoFile), nil
}

func (pr *Prover) checkUpdateNeeded(eki *enclave.EnclaveKeyInfo) bool {
	attestationTime := time.Unix(int64(eki.AttestationTime), 0)

	now := time.Now()
	// TODO consider appropriate buffer time
	// now < attestation_time + expiration / 2
	if now.Before(attestationTime.Add(time.Duration(pr.config.KeyExpiration) * time.Second / 2)) {
		return false
	}
	return true
}

func (pr *Prover) ensureAvailableEnclaveKeyExists(ctx context.Context) error {
	updated, err := pr.updateActiveEnclaveKeyIfNeeded(ctx)
	if err != nil {
		return err
	}
	log.Printf("updateActiveEnclaveKeyIfNeeded: updated=%v", updated)
	return nil
}

// updateActiveEnclaveKeyIfNeeded updates a key if key is missing or expired
func (pr *Prover) updateActiveEnclaveKeyIfNeeded(ctx context.Context) (bool, error) {
	if err := pr.initServiceClient(); err != nil {
		return false, err
	}

	if pr.activeEnclaveKey == nil {
		// load last key if exists
		lastEnclaveKey, err := pr.loadLastEnclaveKey(ctx)
		if err == nil {
			if !pr.checkUpdateNeeded(lastEnclaveKey) {
				pr.activeEnclaveKey = lastEnclaveKey
				return false, nil
			} else {
				log.Printf("last enclave key '0x%x' is found, but needs to be updated", lastEnclaveKey.EnclaveKeyAddress)
			}
		} else if errors.Is(err, ErrLastEnclaveKeyInfoNotFound) {
			log.Printf("last enclave key not found: error=%v", err)
		} else {
			return false, err
		}
	} else if !pr.checkUpdateNeeded(pr.activeEnclaveKey) {
		return false, nil
	}

	log.Println("need to get a new enclave key")

	eki, err := pr.selectNewEnclaveKey(ctx)
	if err != nil {
		return false, err
	}
	log.Printf("selected available enclave key: %#v", eki)
	if err := pr.registerEnclaveKey(eki); err != nil {
		return false, err
	}
	log.Printf("enclave key successfully registered: %#v", eki)
	if err := pr.saveLastEnclaveKey(ctx, eki); err != nil {
		return false, err
	}
	pr.activeEnclaveKey = eki
	return true, nil
}

func (pr *Prover) selectNewEnclaveKey(ctx context.Context) (*enclave.EnclaveKeyInfo, error) {
	res, err := pr.lcpServiceClient.AvailableEnclaveKeys(ctx, &enclave.QueryAvailableEnclaveKeysRequest{Mrenclave: pr.config.GetMrenclave()})
	if err != nil {
		return nil, err
	} else if len(res.Keys) == 0 {
		return nil, fmt.Errorf("no available enclave keys")
	}

	for _, eki := range res.Keys {
		if err := ias.VerifyReport(eki.Report, eki.Signature, eki.SigningCert, time.Now()); err != nil {
			return nil, err
		}
		avr, err := ias.ParseAndValidateAVR(eki.Report)
		if err != nil {
			return nil, err
		}
		if !pr.validateISVEnclaveQuoteStatus(avr.ISVEnclaveQuoteStatus) {
			log.Printf("key '%x' is not allowed to use because of ISVEnclaveQuoteStatus: %v", eki.EnclaveKeyAddress, avr.ISVEnclaveQuoteStatus)
			continue
		}
		if !pr.validateAdvisoryIDs(avr.AdvisoryIDs) {
			log.Printf("key '%x' is not allowed to use because of advisory IDs: %v", eki.EnclaveKeyAddress, avr.AdvisoryIDs)
			continue
		}
		return eki, nil
	}
	return nil, fmt.Errorf("no available enclave keys: all keys are not allowed to use")
}

func (pr *Prover) validateISVEnclaveQuoteStatus(s oias.ISVEnclaveQuoteStatus) bool {
	if s == oias.QuoteOK {
		return true
	}
	for _, status := range pr.config.AllowedQuoteStatuses {
		if s.String() == status {
			return true
		}
	}
	return false
}

func (pr *Prover) validateAdvisoryIDs(ids []string) bool {
	if len(ids) == 0 {
		return true
	}
	allowedSet := mapset.NewSet(pr.config.AllowedAdvisoryIds...)
	targetSet := mapset.NewSet(ids...)
	return targetSet.Difference(allowedSet).Cardinality() == 0
}

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
		anyHeader, err := clienttypes.PackClientMessage(header)
		if err != nil {
			return nil, err
		}
		res, err := pr.lcpServiceClient.UpdateClient(context.TODO(), &elc.MsgUpdateClient{
			ClientId:     pr.config.ElcClientId,
			Header:       anyHeader,
			IncludeState: includeState,
			Signer:       pr.activeEnclaveKey.EnclaveKeyAddress,
		})
		if err != nil {
			return nil, err
		}
		responses = append(responses, res)
	}

	return responses, nil
}

func (pr *Prover) registerEnclaveKey(eki *enclave.EnclaveKeyInfo) error {
	if err := ias.VerifyReport(eki.Report, eki.Signature, eki.SigningCert, time.Now()); err != nil {
		return err
	}
	if _, err := ias.ParseAndValidateAVR(eki.Report); err != nil {
		return err
	}
	message := &lcptypes.RegisterEnclaveKeyMessage{
		Report:      eki.Report,
		Signature:   eki.Signature,
		SigningCert: eki.SigningCert,
	}
	signer, err := pr.originChain.GetAddress()
	if err != nil {
		return err
	}
	msg, err := clienttypes.NewMsgUpdateClient(pr.path.ClientID, message, signer.String())
	if err != nil {
		return err
	}
	if _, err := pr.originChain.SendMsgs([]sdk.Msg{msg}); err != nil {
		return err
	}
	return nil
}

func activateClient(pathEnd *core.PathEnd, src, dst *core.ProvableChain) error {
	srcProver := src.Prover.(*Prover)
	if err := srcProver.ensureAvailableEnclaveKeyExists(context.TODO()); err != nil {
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
		message := &lcptypes.UpdateClientMessage{
			Commitment: update.Commitment,
			Signer:     update.Signer,
			Signature:  update.Signature,
		}
		if err := message.ValidateBasic(); err != nil {
			return err
		}
		msg, err := clienttypes.NewMsgUpdateClient(pathEnd.ClientID, message, signer.String())
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
