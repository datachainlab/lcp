LCP_REPO ?= ..

protoVer=0.13.1
protoImageName=ghcr.io/cosmos/proto-builder:$(protoVer)
protoImage=docker run --user 0 --rm -v $(CURDIR):/workspace --workdir /workspace $(protoImageName)

.PHONY: proto-gen proto-update-deps
proto-gen:
	@echo "Generating Protobuf files"
	@rm -rf ./proto/ibc && rm -rf ./proto/lcp && cp -a $(LCP_REPO)/proto/definitions/ibc ./proto/ && cp -a $(LCP_REPO)/proto/definitions/lcp ./proto/
	@$(protoImage) sh ./scripts/protocgen.sh

proto-update-deps:
	@echo "Updating Protobuf dependencies"
	$(DOCKER) run --user 0 --rm -v $(CURDIR)/proto:/workspace --workdir /workspace $(protoImageName) buf mod update
