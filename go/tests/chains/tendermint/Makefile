include ../../docker.mk

.PHONY: build
build:
	go build -o ./build/simd ./simapp/simd

.PHONY: docker-images
docker-images:
	$(DOCKER_BUILD) \
		--build-arg CHAINID=ibc0 \
		--tag $(DOCKER_REPO)tendermint-chain0:$(DOCKER_TAG) .
	$(DOCKER_BUILD) \
		--build-arg CHAINID=ibc1 \
		--tag $(DOCKER_REPO)tendermint-chain1:$(DOCKER_TAG) .
