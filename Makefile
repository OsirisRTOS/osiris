# Tell make to do as many things as possible at once
ifeq ($(filter -j,$(MAKEFLAGS)),)
	MAKEFLAGS += -j
endif

BUILD_DIR := build
SHELL := /bin/bash
.PHONY: check-format format fmt verify clean hooks

osiris: $(BUILD_DIR)
	cd build && cmake -DMCU=stm32l4r5xx -DBOARD=nucleo -DCMAKE_BUILD_TYPE=Release ..
	cmake --build $(BUILD_DIR) --parallel $(shell nproc)

$(BUILD_DIR):
	@if [ -n "$$CI" ]; then \
		echo "::group::Generating build dir $(BUILD_DIR)"; \
	fi
	cmake -DBOARD=stm32-nucleo-l4r5zi -DCPU=cortex-m4 -B $(BUILD_DIR)
	@if [ -n "$$CI" ]; then \
		echo "::endgroup::"; \
	fi

define ci_check
	@manifests=$$(if [ -z "$(3)" ]; then \
		find . \( -path './Cargo.toml' -o -path './build*' -o -path '*dep*' -o -path '*verus*' -o -path './target' \) -prune -false -o \( -name Cargo.toml -a -not -path './Cargo.toml' \); \
	else \
		echo $(3); \
	fi); \
	failed=0; \
	for manifest in $$(echo $$manifests); do \
		if [ -n "$$CI" ]; then \
			echo "::group::Checking $(1) for $$manifest"; \
		else \
			echo "Checking $(1) for $$manifest"; \
		fi; \
		cargo $(1) --manifest-path="$$manifest" $(2) || failed=1; \
		if [ -n "$$CI" ]; then \
			echo "::endgroup::"; \
		fi; \
	done; \
	if [ $$failed -ne 0 ]; then \
		echo "$(1) check failed for one or more manifests"; \
		exit 1; \
	fi
endef

check-format: $(BUILD_DIR)
	$(call ci_check,fmt,-- --check)

format: fmt
fmt: $(BUILD_DIR)
	$(call ci_check,fmt,)

verify: $(BUILD_DIR)
	$(call ci_check,kani --tests -Z concrete-playback --concrete-playback=print,,kernel/Cargo.toml)

test: $(BUILD_DIR)
	cargo tarpaulin --out Lcov --skip-clean --workspace

watch-tests: $(BUILD_DIR)
	cargo watch --why --exec 'tarpaulin --out Lcov --skip-clean --workspace' --ignore lcov.info

clean:
	rm -rf $(BUILD_DIR)

hooks:
	ln -sf $(CURDIR)/.devcontainer/pre-commit.sh $(CURDIR)/.git/hooks/pre-commit

check: check-format test verify
	echo "All checks passed"
