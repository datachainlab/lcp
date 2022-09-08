DOCKER         ?= docker
DOCKER_COMPOSE ?= docker-compose
DOCKER_REPO    ?= ""
DOCKER_TAG     ?= latest
DOCKER_BUILD   ?= $(DOCKER) build --rm --no-cache --pull

RLY_BIN ?= ../../../../go/build/yrly

.PHONY: network
network:
	TAG=${DOCKER_TAG} $(DOCKER_COMPOSE) \
		-f ../docker-compose-test.yaml \
		up -d \
		tendermint-chain0 tendermint-chain1

.PHONY: network-down
network-down:
	TAG=${DOCKER_TAG} $(DOCKER_COMPOSE) \
		-f ../docker-compose-test.yaml \
		down --volume --remove-orphans

.PHONY: test
test:
	./scripts/fixture
	RLY_BIN=$(RLY_BIN) ./scripts/init-rly
	RLY_BIN=$(RLY_BIN) ./scripts/handshake
	RLY_BIN=$(RLY_BIN) ./scripts/test-tx
