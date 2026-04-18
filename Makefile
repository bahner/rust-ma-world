.DEFAULT_GOAL := all

TARGET ?= x86_64-unknown-linux-musl
PACKAGE ?= ma-world
PROFILE ?= release
BIN_PATH := target/$(TARGET)/$(PROFILE)/$(PACKAGE)
RUSTUP_STAMP := target/.rustup-target-$(TARGET).stamp

WORLD_SOURCES := $(wildcard world/src/*.rs)
CORE_SOURCES := $(wildcard ma-world-core/src/*.rs)
MANIFESTS := Cargo.toml Cargo.lock world/Cargo.toml ma-world-core/Cargo.toml

.PHONY: all check clean distclean deploy

all: $(BIN_PATH)

$(RUSTUP_STAMP):
	@mkdir -p $(dir $@)
	rustup target add $(TARGET)
	@touch $@

build: $(BIN_PATH)

$(BIN_PATH): $(WORLD_SOURCES) $(CORE_SOURCES) $(MANIFESTS) | $(RUSTUP_STAMP)
	cargo build -p $(PACKAGE) --$(PROFILE) --target $(TARGET)

check: $(WORLD_SOURCES) $(CORE_SOURCES) $(MANIFESTS) | $(RUSTUP_STAMP)
	cargo check -p $(PACKAGE) --target $(TARGET)

deploy: $(BIN_PATH)
	scp ./$(BIN_PATH) ma-world:bin/

clean:
	cargo clean -p $(PACKAGE)
	rm -f $(RUSTUP_STAMP)

distclean:
	cargo clean
