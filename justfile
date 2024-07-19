export NAME := 'cosmic-launcher'
export APPID := 'com.system76.CosmicLauncher'

rootdir := ''
prefix := '/usr'
debug := '0'

base-dir := absolute_path(clean(rootdir / prefix))

export INSTALL_DIR := base-dir / 'share'

cargo-target-dir := env('CARGO_TARGET_DIR', 'target')
bin-src := if debug == '1' { cargo-target-dir / 'target/debug' / NAME } else { cargo-target-dir / 'target/release' / NAME }
bin-dst := base-dir / 'bin' / NAME

# Use mold linker if clang and mold exists.
clang-path := `which clang || true`
mold-path := `which mold || true`

ld-args := if clang-path != '' {
    if mold-path != '' {
        '-C linker=' + clang-path + ' -C link-arg=--ld-path=' + mold-path + ' '
    } else {
        ''
    }
} else {
    ''
}

export RUSTFLAGS := env_var_or_default('RUSTFLAGS', '') + ' --cfg tokio_unstable ' + ld-args

# Default recipe which runs `just build-release`
default: build-release

# Runs `cargo clean`
clean:
    cargo clean

# `cargo clean` and removes vendored dependencies
clean-dist: clean
    rm -rf vendor vendor.tar

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

# Runs after compiling a release build
run: build-release
    {{bin-src}}

# Build and run with tokio-console enabled
tokio-console: (build-release '--features console')
    env TOKIO_CONSOLE=1 {{bin-src}}

# Installs files
install:
    install -Dm0755 {{bin-src}} {{bin-dst}}
    @just data/install
    @just data/icons/install

# Uninstalls installed files
uninstall:
    rm {{bin-dst}}
    @just data/uninstall
    @just data/icons/uninstall

# Vendor dependencies locally
vendor:
    cp .cargo/config.default .cargo/config.toml
    cargo vendor --sync Cargo.toml \
        | head -n -1 >> .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    rm -rf vendor/winapi*gnu*/lib/*.a; \
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
vendor-extract:
    #!/usr/bin/env sh
    rm -rf vendor
    tar pxf vendor.tar
