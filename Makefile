######## LCP Build Settings ########
LCP_RISC0_BUILD ?= 0
ZK_PROVER_CUDA ?= 0
APP_CARGO_FLAGS ?=

######## SGX SDK Settings ########
SGX_SDK ?= /opt/sgxsdk
SGX_MODE ?= HW
SGX_ARCH ?= x64
SGX_DEBUG ?= 0
SGX_PRERELEASE ?= 0
SGX_PRODUCTION ?= 0

include buildenv.mk

ifeq ($(shell getconf LONG_BIT), 32)
	SGX_ARCH := x86
else ifeq ($(findstring -m32, $(CXXFLAGS)), -m32)
	SGX_ARCH := x86
endif

ifeq ($(SGX_ARCH), x86)
	SGX_COMMON_CFLAGS := -m32
	SGX_LIBRARY_PATH := $(SGX_SDK)/lib
	SGX_ENCLAVE_SIGNER := $(SGX_SDK)/bin/x86/sgx_sign
	SGX_EDGER8R := $(SGX_SDK)/bin/x86/sgx_edger8r
else
	SGX_COMMON_CFLAGS := -m64
	SGX_LIBRARY_PATH := $(SGX_SDK)/lib64
	SGX_ENCLAVE_SIGNER := $(SGX_SDK)/bin/x64/sgx_sign
	SGX_EDGER8R := $(SGX_SDK)/bin/x64/sgx_edger8r
endif

ifeq ($(SGX_DEBUG), 1)
ifeq ($(SGX_PRERELEASE), 1)
$(error Cannot set SGX_DEBUG and SGX_PRERELEASE at the same time!!)
endif
ifeq ($(SGX_PRODUCTION), 1)
$(error Cannot set SGX_DEBUG and SGX_PRODUCTION at the same time!!)
endif
endif

ifeq ($(SGX_DEBUG), 1)
	# we build with cargo --release, even in SGX DEBUG mode
	SGX_COMMON_CFLAGS += -O0 -g -ggdb
	# cargo sets this automatically, cannot use 'debug'
	OUTPUT_PATH := release
	CARGO_TARGET := --release
else
	SGX_COMMON_CFLAGS += -O2
	OUTPUT_PATH := release
	CARGO_TARGET := --release
endif

SGX_COMMON_CFLAGS += -fstack-protector

APP_CARGO_FEATURES = --features=default
SGX_ENCLAVE_CONFIG = "enclave/Enclave.config.xml"
ifeq ($(SGX_PRODUCTION), 1)
	SGX_ENCLAVE_MODE = "Production Mode"
	SGX_ENCLAVE_CONFIG = $(SGX_ENCLAVE_CONFIG)
	SGX_SIGN_KEY = $(SGX_COMMERCIAL_KEY)
else
	SGX_ENCLAVE_MODE = "Development Mode"
	SGX_SIGN_KEY = "enclave/Enclave_private.pem"
	ifneq ($(SGX_MODE), HW)
		APP_CARGO_FEATURES     = --features=default,sgx-sw
	endif
endif
ifeq ($(ZK_PROVER_CUDA), 1)
	APP_CARGO_FEATURES     = --features=default,cuda
endif

######## CUSTOM Settings ########

CUSTOM_LIBRARY_PATH := ./lib
CUSTOM_BIN_PATH := ./bin

######## EDL Settings ########

Enclave_EDL_Files := enclave/Enclave_t.c enclave/Enclave_t.h app/Enclave_u.c app/Enclave_u.h

######## APP Settings ########

App_Rust_Flags := $(CARGO_TARGET) $(APP_CARGO_FEATURES) $(APP_CARGO_FLAGS)
App_SRC_Files := $(shell find app/ -type f -name '*.rs') $(shell find app/ -type f -name 'Cargo.toml')
App_Include_Paths := -I ./app -I./include -I$(SGX_SDK)/include
App_C_Flags := $(SGX_COMMON_CFLAGS) -fPIC -Wno-attributes $(App_Include_Paths)

App_Rust_Path := ./target/$(OUTPUT_PATH)
App_Enclave_u_Object :=lib/libEnclave_u.a
App_Name := lcp
App_Dir := ./bin
App_Path := $(App_Dir)/$(App_Name)

######## Enclave Settings ########

ifneq ($(SGX_MODE), HW)
	Trts_Library_Name := sgx_trts_sim
	Service_Library_Name := sgx_tservice_sim
else
	Trts_Library_Name := sgx_trts
	Service_Library_Name := sgx_tservice
endif
Crypto_Library_Name := sgx_tcrypto
ProtectedFs_Library_Name := sgx_tprotected_fs

RustEnclave_C_Files := $(wildcard ./enclave/*.c)
RustEnclave_C_Objects := $(RustEnclave_C_Files:.c=.o)
RustEnclave_Include_Paths := -I$(SGX_SDK)/include -I$(SGX_SDK)/include/tlibc -I$(SGX_SDK)/include/stlport -I$(SGX_SDK)/include/epid -I ./enclave -I./include

RustEnclave_Link_Libs := -L$(CUSTOM_LIBRARY_PATH) -lenclave
RustEnclave_Compile_Flags := $(SGX_COMMON_CFLAGS) $(ENCLAVE_CFLAGS) $(RustEnclave_Include_Paths)
RustEnclave_Link_Flags := -Wl,--no-undefined -nostdlib -nodefaultlibs -nostartfiles -L$(SGX_LIBRARY_PATH) \
	-Wl,--whole-archive -l$(Trts_Library_Name) -l${ProtectedFs_Library_Name} -Wl,--no-whole-archive \
	-Wl,--start-group -lsgx_tcxx -lsgx_tstdc -l$(Service_Library_Name) -l$(Crypto_Library_Name) $(RustEnclave_Link_Libs) -Wl,--end-group \
	-Wl,--version-script=enclave/Enclave.lds \
	$(ENCLAVE_LDFLAGS)

RustEnclave_Name := enclave/enclave.so
Signed_RustEnclave_Name := bin/enclave.signed.so

######## Test Settings ########

GAIAD_VERSION ?= v7.0.3

######## Targets ########

.PHONY: all
all: $(App_Path) $(Signed_RustEnclave_Name)

.PHONY: clean
clean:
	@rm -f $(App_Dir)/* $(RustEnclave_Name) $(Signed_RustEnclave_Name) enclave/*_t.* app/*_u.* lib/*.a
	@cargo clean
	@cd enclave && cargo clean
	@cd enclave-modules && cargo clean

######## EDL Objects ########

$(Enclave_EDL_Files): $(SGX_EDGER8R) enclave/Enclave.edl
	$(SGX_EDGER8R) --trusted enclave/Enclave.edl --search-path $(SGX_SDK)/include --trusted-dir enclave
	$(SGX_EDGER8R) --untrusted enclave/Enclave.edl --search-path $(SGX_SDK)/include --untrusted-dir app
	@echo "GEN  =>  $(Enclave_EDL_Files)"

######## App Objects ########

app/Enclave_u.o: $(Enclave_EDL_Files)
	@$(CC) $(App_C_Flags) -c app/Enclave_u.c -o $@
	@echo "CC   <=  $<"

$(App_Enclave_u_Object): app/Enclave_u.o
	$(AR) rcsD $@ $^

$(App_Path): $(App_Enclave_u_Object) $(App_SRC_Files)
	@cd app && SGX_SDK=$(SGX_SDK) SGX_MODE=$(SGX_MODE) LCP_RISC0_BUILD=$(LCP_RISC0_BUILD) cargo build $(App_Rust_Flags)
	@echo "Cargo  =>  $@"
	mkdir -p bin
	cp $(App_Rust_Path)/$(App_Name) ./bin

######## Enclave Objects ########

enclave/Enclave_t.o: $(Enclave_EDL_Files)
	@$(CC) $(RustEnclave_Compile_Flags) -c enclave/Enclave_t.c -o $@
	@echo "CC   <=  $<"

$(RustEnclave_Name): enclave enclave/Enclave_t.o
	@$(CXX) enclave/Enclave_t.o -o $@ $(RustEnclave_Link_Flags)
	@echo "LINK =>  $@"

$(Signed_RustEnclave_Name): $(RustEnclave_Name)
	mkdir -p bin
	@$(SGX_ENCLAVE_SIGNER) sign -key enclave/Enclave_private.pem -enclave $(RustEnclave_Name) -out $@ -config $(SGX_ENCLAVE_CONFIG)
	@echo "SIGN =>  $@"

.PHONY: enclave
enclave:
	@cd enclave && RUSTFLAGS=$(RUSTFLAGS) cargo build $(CARGO_TARGET)
	@cp enclave/target/$(OUTPUT_PATH)/libproxy_enclave.a ./lib/libenclave.a

######## Code generator ########

.PHONY: proto
proto:
	@cd proto-compiler && cargo run -- compile --ibc /tmp/cosmos/ibc --out ../proto/src/prost --descriptor ../proto/src/descriptor.bin

######## Lint ########

.PHONY: lint-tools
lint-tools:
	rustup component add rustfmt clippy
	cargo +nightly install cargo-machete

.PHONY: fmt
fmt:
	@cargo fmt --all $(CARGO_FMT_OPT)
	@$(TEST_ENCLAVE_CARGO) fmt --all $(CARGO_FMT_OPT)
	@cd ./enclave && cargo fmt --all $(CARGO_FMT_OPT)

.PHONY: lint
lint:
	@$(MAKE) CARGO_FMT_OPT=--check fmt
	@cargo clippy --locked --tests $(CARGO_TARGET) -- -D warnings
	@$(TEST_ENCLAVE_CARGO) clippy --locked --tests $(CARGO_TARGET) -- -D warnings
	@cargo machete

.PHONY: udeps
udeps:
	@cargo +nightly install cargo-udeps --locked
	@cargo +nightly udeps --locked --lib --tests $(CARGO_TARGET)

######## Tools ########

.PHONY: nodes-runner
nodes-runner:
	@cargo build $(CARGO_TARGET) --package nodes-runner

######## Tests ########

TEST_ENCLAVE_RUSTFLAGS="-L $(SGX_SDK)/lib64"
TEST_ENCLAVE_CARGO=RUSTFLAGS=$(TEST_ENCLAVE_RUSTFLAGS) cargo -Z unstable-options -C enclave-modules
TEST_ENCLAVE_CARGO_TEST=$(TEST_ENCLAVE_CARGO) test $(CARGO_TARGET)

.PHONY: test
test:
	@cargo test $(CARGO_TARGET) --workspace --exclude integration-test
	@$(TEST_ENCLAVE_CARGO_TEST) -p ecall-handler
	@$(TEST_ENCLAVE_CARGO_TEST) -p enclave-environment
	@$(TEST_ENCLAVE_CARGO_TEST) -p host-api
	@$(TEST_ENCLAVE_CARGO_TEST) -p enclave-runtime
	@$(TEST_ENCLAVE_CARGO_TEST) -p enclave-utils

.PHONY: integration-test
integration-test: $(Signed_RustEnclave_Name) bin/gaiad
	@PATH=${PATH}:$(CURDIR)/bin cargo test $(CARGO_TARGET) --package integration-test $(APP_CARGO_FEATURES)

.PHONY: test-nodes
test-setup-nodes: bin/gaiad
	@PATH=${PATH}:$(CURDIR)/bin cargo run --bin test_setup_with_binary_channel

bin/gaiad:
	curl -o ./bin/gaiad -LO https://github.com/cosmos/gaia/releases/download/$(GAIAD_VERSION)/gaiad-$(GAIAD_VERSION)-linux-amd64 && chmod +x ./bin/gaiad
