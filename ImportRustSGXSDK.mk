# helper script to fetch the files in rust-sgx-sdk to the target version

GIT = git
CP  = cp

REPO = https://github.com/apache/incubator-teaclave-sgx-sdk
SDK_PATH_GIT = rust-sgx-sdk-github
SDK_PATH = rust-sgx-sdk
VERSION_FILE = rust-sgx-sdk/version
TARGET_VERSION = $(shell cat $(VERSION_FILE))
COMMAND = git ls-remote $(REPO) HEAD | awk '{ print $$1 }'

updatesdk:
	@echo Target version = $(TARGET_VERSION)

	@rm -rf $(SDK_PATH_GIT)
	@$(GIT) clone $(REPO) $(SDK_PATH_GIT)
	@cd $(SDK_PATH_GIT) && $(GIT) checkout $(TARGET_VERSION) && cd ..
	rsync -a $(SDK_PATH_GIT)/edl $(SDK_PATH)
	rsync -a $(SDK_PATH_GIT)/common $(SDK_PATH)
	rsync -a $(SDK_PATH_GIT)/dockerfile $(SDK_PATH)
	rm -rf $(SDK_PATH_GIT)
