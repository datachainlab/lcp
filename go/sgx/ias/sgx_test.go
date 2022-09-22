package ias

import (
	"encoding/json"
	"io/ioutil"
	"testing"
	"time"

	"github.com/ethereum/go-ethereum/common"
	"github.com/oasisprotocol/oasis-core/go/common/sgx/ias"
	"github.com/stretchr/testify/require"
)

type EndorsedAttestationVerificationReport struct {
	AVR         string `json:"avr"`
	Signature   []byte `json:"signature"`
	SigningCert []byte `json:"signing_cert"`
}

func TestReportVerification(t *testing.T) {
	ias.SetAllowDebugEnclaves()
	defer ias.UnsetAllowDebugEnclaves()

	const path = "../../testdata/avr-280814000042567078634030940835907407639"
	bz, err := ioutil.ReadFile(path)
	require.NoError(t, err)

	var eavr EndorsedAttestationVerificationReport
	require.NoError(t, json.Unmarshal(bz, &eavr))

	require.NoError(t, VerifyReport(eavr.AVR, eavr.Signature, eavr.SigningCert, time.Now()))
	avr, err := ParseAndValidateAVR(eavr.AVR)
	require.NoError(t, err)

	quote, err := avr.Quote()
	require.NoError(t, err)
	addr, err := GetEnclaveKeyAddress(quote)
	require.NoError(t, err)
	require.Equal(t, common.HexToAddress("0x0f1bf704ad5bbadb4ae8a2a8d4ecb1999a1e05ed"), addr)
}
