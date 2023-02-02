name := 'cosmic-launcher'
export APPID := 'com.system76.CosmicLauncher'

rootdir := ''
prefix := '/usr'

export INSTALL_DIR := clean(justfile_directory() / rootdir / prefix / 'share')

bin-src := 'target' / 'release' / name
bin-dst := INSTALL_DIR / 'bin' / name

# Default recipe which runs `just build-release`
default: build-release

# Runs `cargo clean`
clean:
    cargo clean

# `cargo clean` and removes vendored dependencies
clean-dist: clean
    rm -rf .cargo vendor vendor.tar

# Compiles with debug profile
build-debug *args:
    cargo build {{args}}

# Compiles with release profile
build-release *args: (build-debug '--release' args)

# Compiles release profile with vendored dependencies
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

# Runs a clippy check
check *args:
    cargo clippy --all-features {{args}} -- -W clippy::pedantic

# Runs a clippy check with JSON message format
check-json: (check '--message-format=json')

# Installs files
install:
    install -Dm0644 {{bin-src}} {{bin-dst}}
    @just data/install
    @just data/icons/install

# Uninstalls installed files
uninstall:
    rm {{bin-dst}}
    @just data/uninstall
    @just data/icons/uninstall

# Vendor dependencies locally
vendor:
    mkdir -p .cargo
    cargo vendor --sync Cargo.toml \
        | head -n -1 > .cargo/config
    echo 'directory = "vendor"' >> .cargo/config
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
vendor-extract:
    #!/usr/bin/env sh
    rm -rf vendor
    tar pxf vendor.tar
