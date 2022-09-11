package ias

import "github.com/oasisprotocol/oasis-core/go/common/sgx/ias"

// SetAllowDebugEnclave will enable running and communicating with enclaves
// with debug flag enabled in AVR for the remainder of the process' lifetime.
func SetAllowDebugEnclaves() {
	ias.SetAllowDebugEnclaves()
}

// UnsetAllowDebugEnclave will disable running and communicating with enclaves
// with debug flag enabled in AVR for the remainder of the process' lifetime.
func UnsetAllowDebugEnclaves() {
	ias.UnsetAllowDebugEnclaves()
}
