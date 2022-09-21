LCP_REPO ?= ..

.PHONY: proto-gen
proto-gen:
	@echo "Generating Protobuf files"
	@rm -rf ./proto/ibc && rm -rf ./proto/lcp && cp -a $(LCP_REPO)/proto/definitions/ibc ./proto/ && cp -a $(LCP_REPO)/proto/definitions/lcp ./proto/
	docker run -v $(CURDIR):/workspace --workdir /workspace tendermintdev/sdk-proto-gen:v0.3 sh ./scripts/protocgen.sh
