name := 'cosmic-launcher'
export APPID := 'com.system76.CosmicLauncher'

rootdir := ''
prefix := '/usr'

base-dir := absolute_path(clean(rootdir / prefix))

export INSTALL_DIR := base-dir / 'share'

bin-src := 'target' / 'release' / name
bin-dst := base-dir / 'bin' / name

# Use lld linker if available
ld-args := if `which lld || true` != '' {
    '-C link-arg=-fuse-ld=lld -C link-arg=-Wl,--build-id=sha1 -Clink-arg=-Wl,--no-rosegment'
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
    ./target/release/cosmic-launcher

# Build and run with tokio-console enabled
tokio-console: build-release
    env TOKIO_CONSOLE=1 ./target/release/cosmic-launcher

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
