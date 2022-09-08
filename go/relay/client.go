package relay

import (
	"github.com/datachainlab/lcp/go/relay/elc"
	"github.com/datachainlab/lcp/go/relay/enclave"
	"google.golang.org/grpc"
)

type LCPServiceClient struct {
	elc.MsgClient
	enclave.QueryClient
}

func NewLCPServiceClient(conn *grpc.ClientConn) LCPServiceClient {
	return LCPServiceClient{
		MsgClient:   elc.NewMsgClient(conn),
		QueryClient: enclave.NewQueryClient(conn),
	}
}
