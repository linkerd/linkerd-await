TARGET ?= x86_64-unknown-linux-musl
TARGET_DIR = target/$(TARGET)/debug
ifdef CARGO_RELEASE
	RELEASE = --release
	TARGET_DIR = target/$(TARGET)/release
endif
TARGET_BIN = $(TARGET_DIR)/linkerd-await

RUSTUP ?= rustup
CARGO ?= cargo
CARGO_BUILD = $(CARGO) build --frozen --target=$(TARGET) $(RELEASE)
CARGO_CHECK = $(CARGO) check --all --frozen --target=$(TARGET) $(RELEASE)
CARGO_TEST = $(CARGO) test --all --frozen --target=$(TARGET) $(RELEASE)
CARGO_FMT = $(CARGO) fmt --all

SHASUM = shasum -a 256

.PHONY: all
all: check-fmt check

.PHONY: configure-target
configure-target:
	$(RUSTUP) target add $(TARGET)

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

release: $(TARGET_BIN)
	@rm -rf release && mkdir release
	cp $(TARGET_BIN) release/linkerd-await
	strip release/linkerd-await
	$(SHASUM) release/linkerd-await >release/linkerd-await.shasum
