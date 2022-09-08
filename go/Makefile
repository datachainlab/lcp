LCP_REPO ?= ..

.PHONY: yrly
yrly:
	go build -o ./build/yrly ./relay/bin/main.go

.PHONY: proto-gen
proto-gen:
	@echo "Generating Protobuf files"
	@rm -rf ./proto/ibc && rm -rf ./proto/lcp && cp -a $(LCP_REPO)/proto/definitions/ibc ./proto/ && cp -a $(LCP_REPO)/proto/definitions/lcp ./proto/
	docker run -v $(CURDIR):/workspace --workdir /workspace tendermintdev/sdk-proto-gen:v0.3 sh ./scripts/protocgen.sh
