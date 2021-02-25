RUSTUP ?= rustup
CARGO_TARGET ?= $(shell $(RUSTUP) show |sed -n 's/^Default host: \(.*\)/\1/p')
TARGET_DIR = target/$(CARGO_TARGET)/debug
ifdef CARGO_RELEASE
	RELEASE = --release
	TARGET_DIR = target/$(CARGO_TARGET)/release
endif
TARGET_BIN = $(TARGET_DIR)/linkerd-await

ARCH ?= amd64
STRIP ?= strip

PKG_ROOT = $(TARGET)/package
PKG_NAME = linkerd-await-$(PACKAGE_VERSION)-$(ARCH)
PKG_BASE = $(PKG_ROOT)/$(PKG_NAME)

SHASUM = shasum -a 256

CARGO ?= cargo
CARGO_BUILD = $(CARGO) build --frozen --target=$(CARGO_TARGET) $(RELEASE)
CARGO_CHECK = $(CARGO) check --all --frozen --target=$(CARGO_TARGET) $(RELEASE)
CARGO_TEST = $(CARGO) test --all --frozen --target=$(CARGO_TARGET) $(RELEASE)
CARGO_FMT = $(CARGO) fmt --all

.PHONY: all
all: check-fmt check

.PHONY: configure-target
configure-target:
	$(RUSTUP) target add $(CARGO_TARGET)

.PHONY: configure-fmt
configure-fmt:
	$(RUSTUP) component add rustfmt

.PHONY: fetch
fetch: Cargo.lock
	$(CARGO) fetch --locked

$(TARGET_BIN): fetch configure-target
	$(CARGO_BUILD)

.PHONY: clean
clean:
	$(CARGO) clean --target-dir $(TARGET_DIR)

.PHONY: check-fmt
check-fmt: configure-fmt
	$(CARGO_FMT) -- --check

.PHONY: check
check: configure-target fetch
	$(CARGO_CHECK)


.PHONY: fmt
fmt: configure-fmt
	$(CARGO_FMT)

.PHONY: build
build: $(TARGET_BIN)

.PHONY: release
release: $(TARGET_BIN)
	@mkdir -p release
	cp $(TARGET_BIN) release/$(PKG_NAME)
	$(STRIP) release/$(PKG_NAME)
	$(SHASUM) release/$(PKG_NAME) >release/$(PKG_NAME).shasum
