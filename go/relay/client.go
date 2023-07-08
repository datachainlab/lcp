package relay

import (
	"github.com/cosmos/cosmos-sdk/codec"
	"github.com/datachainlab/lcp/go/relay/elc"
	"github.com/datachainlab/lcp/go/relay/enclave"
	"google.golang.org/grpc"
)

type (
	ELCMsgClient       = elc.MsgClient
	ELCQueryClient     = elc.QueryClient
	EnclaveQueryClient = enclave.QueryClient
)

type LCPServiceClient struct {
	codec codec.ProtoCodecMarshaler

	ELCMsgClient
	ELCQueryClient
	EnclaveQueryClient
}

func NewLCPServiceClient(conn *grpc.ClientConn, codec codec.ProtoCodecMarshaler) LCPServiceClient {
	return LCPServiceClient{
		codec:              codec,
		ELCMsgClient:       elc.NewMsgClient(conn),
		ELCQueryClient:     elc.NewQueryClient(conn),
		EnclaveQueryClient: enclave.NewQueryClient(conn),
	}
}
