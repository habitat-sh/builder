UNAME_S := $(shell uname -s)
BIN =
LIB = builder-db builder-core github-api-client
SRV = builder-api builder-jobsrv builder-worker
ALL = $(BIN) $(LIB) $(SRV)

.DEFAULT_GOAL := build-bin

linux:
ifeq ($(UNAME_S),Darwin)
	$(error Please run this from Linux)
endif
.PHONY: linux

build: build-bin build-lib build-srv ## builds all the components
build-all: build
.PHONY: build build-all

build-bin: $(addprefix build-,$(BIN)) ## builds the binary components
.PHONY: build-bin

build-lib: $(addprefix build-,$(LIB)) ## builds the library components
.PHONY: build-lib

build-srv: $(addprefix build-,$(SRV)) ## builds the service components
.PHONY: build-srv

unit: unit-bin unit-lib unit-srv ## executes all the components' unit test suites
unit-all: unit
.PHONY: unit unit-all

unit-bin: $(addprefix unit-,$(BIN)) ## executes the binary components' unit test suites
.PHONY: unit-bin

unit-lib: $(addprefix unit-,$(LIB)) ## executes the library components' unit test suites
.PHONY: unit-lib

unit-srv: $(addprefix unit-,$(SRV)) ## executes the service components' unit test suites
.PHONY: unit-srv

lint: lint-bin lint-lib lint-srv ## executs all components' lints
lint-all: lint
.PHONY: lint lint-all

lint-bin: $(addprefix lint-,$(BIN))
.PHONY: lint-bin

lint-lib: $(addprefix lint-,$(LIB))
.PHONY: lint-lib

lint-srv: $(addprefix lint-,$(SRV))
.PHONY: lint-srv

functional: functional-bin functional-lib functional-srv ## executes all the components' functional test suites
functional-all: functional
test: functional ## executes all components' test suites
.PHONY: functional functional-all test

functional-bin: $(addprefix unit-,$(BIN)) ## executes the binary components' unit functional suites
.PHONY: functional-bin

functional-lib: $(addprefix unit-,$(LIB)) ## executes the library components' unit functional suites
.PHONY: functional-lib

functional-srv: $(addprefix unit-,$(SRV)) ## executes the service components' unit functional suites
.PHONY: functional-srv

clean: clean-bin clean-lib clean-srv ## cleans all the components' clean test suites
clean-all: clean
.PHONY: clean clean-all

clean-bin: $(addprefix clean-,$(BIN)) ## cleans the binary components' project trees
.PHONY: clean-bin

clean-lib: $(addprefix clean-,$(LIB)) ## cleans the library components' project trees
.PHONY: clean-lib

clean-srv: $(addprefix clean-,$(SRV)) ## cleans the service components' project trees
.PHONY: clean-srv

fmt: fmt-bin fmt-lib fmt-srv ## formats all the components' codebases
fmt-all: fmt
.PHONY: fmt fmt-all

fmt-bin: $(addprefix fmt-,$(BIN)) ## formats the binary components' codebases
.PHONY: clean-bin

fmt-lib: $(addprefix fmt-,$(LIB)) ## formats the library components' codebases
.PHONY: clean-lib

fmt-srv: $(addprefix fmt-,$(SRV)) ## formats the service components' codebases
.PHONY: clean-srv

help:
	@perl -nle'print $& if m{^[a-zA-Z_-]+:.*?## .*$$}' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'
.PHONY: help

define BUILD
build-$1: linux ## builds the $1 component
	sh -c 'cd components/$1 && cargo build'
.PHONY: build-$1

endef
$(foreach component,$(ALL),$(eval $(call BUILD,$(component))))

define UNIT
unit-$1: linux ## executes the $1 component's unit test suite
	sh -c 'cd components/$1 && cargo test'
.PHONY: unit-$1
endef
$(foreach component,$(ALL),$(eval $(call UNIT,$(component))))

define LINT
lint-$1: linux ## executes the $1 component's linter checks
	sh -c 'cd components/$1 && cargo build --features clippy'
.PHONY: lint-$1
endef
$(foreach component,$(ALL),$(eval $(call LINT,$(component))))

define FUNCTIONAL
functional-$1: linux ## executes the $1 component's functional test suite
	sh -c 'cd components/$1 && cargo test --features functional'
.PHONY: functional-$1

endef
$(foreach component,$(ALL),$(eval $(call FUNCTIONAL,$(component))))

define CLEAN
clean-$1: linux ## cleans the $1 component's project tree
	sh -c 'cd components/$1 && cargo clean'
.PHONY: clean-$1

endef
$(foreach component,$(ALL),$(eval $(call CLEAN,$(component))))

define FMT
fmt-$1: linux ## formats the $1 component
	sh -c 'cd components/$1 && cargo fmt'
.PHONY: fmt-$1

endef
$(foreach component,$(ALL),$(eval $(call FMT,$(component))))
