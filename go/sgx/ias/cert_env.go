//go:build customcert

package ias

import (
	"encoding/hex"
	"fmt"
	"os"

	"github.com/oasisprotocol/oasis-core/go/common/sgx/ias"
)

const envRARootCert = "LCP_RA_ROOT_CERT_HEX"

func init() {
	cert := os.Getenv(envRARootCert)
	if len(cert) == 0 {
		initIAS()
	} else {
		initFromEnv(cert)
	}
}

func initFromEnv(cert string) {
	pem, err := hex.DecodeString(cert)
	if err != nil {
		panic(err)
	}
	rootCert, _, err := ias.CertFromPEM(pem)
	if err != nil {
		panic(err)
	} else if rootCert == nil {
		panic(fmt.Sprintf("invalid rootCert: %v", cert))
	}
	setRARootCert(rootCert)
}
