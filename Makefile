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

# Lints we need to work through and decide as a team whether to allow or fix
UNEXAMINED_LINTS =

# Lints we disagree with and choose to keep in our code with no warning
ALLOWED_LINTS = clippy::module_inception \
                clippy::new_ret_no_self \
                clippy::new_without_default

# Known failing lints we want to receive warnings for, but not fail the build
LINTS_TO_FIX =

# Lints we don't expect to have in our code at all and want to avoid adding
# even at the cost of failing the build
DENIED_LINTS = clippy::assign_op_pattern \
               clippy::blacklisted_name \
               clippy::block_in_if_condition_stmt \
               clippy::bool_comparison \
               clippy::cast_lossless \
               clippy::clone_on_copy \
               clippy::cmp_owned \
               clippy::collapsible_if \
               clippy::const_static_lifetime \
               clippy::correctness \
               clippy::cyclomatic_complexity \
               clippy::deref_addrof \
               clippy::expect_fun_call \
               clippy::for_kv_map \
               clippy::get_unwrap \
               clippy::identity_conversion \
               clippy::if_let_some_result \
               clippy::large_enum_variant \
               clippy::len_without_is_empty \
               clippy::len_zero \
               clippy::let_and_return \
               clippy::let_unit_value \
               clippy::map_clone \
               clippy::match_bool \
               clippy::match_ref_pats \
               clippy::needless_bool \
               clippy::needless_collect \
               clippy::needless_pass_by_value \
               clippy::needless_range_loop \
               clippy::needless_return \
               clippy::needless_update \
               clippy::ok_expect \
               clippy::op_ref \
               clippy::option_map_unit_fn \
               clippy::or_fun_call \
               clippy::println_empty_string \
               clippy::ptr_arg \
               clippy::question_mark \
               clippy::redundant_closure \
               clippy::redundant_field_names \
               clippy::redundant_pattern_matching \
               clippy::single_char_pattern \
               clippy::single_match \
               clippy::string_lit_as_bytes \
               clippy::too_many_arguments \
               clippy::toplevel_ref_arg \
               clippy::trivially_copy_pass_by_ref \
               clippy::unit_arg \
               clippy::unnecessary_operation \
               clippy::unreadable_literal \
               clippy::unused_label \
               clippy::unused_unit \
               clippy::useless_asref \
               clippy::useless_format \
               clippy::useless_let_if_seq \
               clippy::useless_vec \
               clippy::write_with_newline \
               clippy::wrong_self_convention \
               renamed_and_removed_lints

lint:
	$(run) cargo clippy --all-targets --tests $(CARGO_FLAGS) -- \
					$(addprefix -A ,$(UNEXAMINED_LINTS)) \
					$(addprefix -A ,$(ALLOWED_LINTS)) \
					$(addprefix -W ,$(LINTS_TO_FIX)) \
					$(addprefix -D ,$(DENIED_LINTS))
.PHONY: lint

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
