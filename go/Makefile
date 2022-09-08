LCP_REPO ?= ..
DOCKER         ?= docker
DOCKER_BUILD   ?= $(DOCKER) build --rm --no-cache --pull

.PHONY: yrly
yrly:
	go build -o ./build/yrly ./relay/bin/main.go

.PHONY: proto-gen
proto-gen:
	@echo "Generating Protobuf files"
	@rm -rf ./proto/ibc && rm -rf ./proto/lcp && cp -a $(LCP_REPO)/proto/definitions/ibc ./proto/ && cp -a $(LCP_REPO)/proto/definitions/lcp ./proto/
	docker run -v $(CURDIR):/workspace --workdir /workspace tendermintdev/sdk-proto-gen:v0.3 sh ./scripts/protocgen.sh

.PHONY: tendermint-docker
tendermint-docker:
	$(DOCKER_BUILD) --build-arg CHAINID=ibc0 --tag tendermint-chain0 -f ./simapp/Dockerfile .
	$(DOCKER_BUILD) --build-arg CHAINID=ibc1 --tag tendermint-chain1 -f ./simapp/Dockerfile .
