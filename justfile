mod cargo 'cargo.just'

name := 'cosmic-launcher'
appid := 'com.system76.CosmicLauncher'
export APP_ID := appid

rootdir := ''
prefix := '/usr'
debug := '0'

appdata := appid + '.metainfo.xml'
desktop := appid + '.desktop'

base-dir := absolute_path(clean(rootdir / prefix))
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')
bin-src := if debug == '1' { 'debug' / name } else { cargo-target-dir / 'release' / name }
bin-dst := base-dir / 'bin' / name
appdata-dst := base-dir / 'share' / 'appdata' / appdata
desktop-dst := base-dir / 'share' / 'applications' / desktop
icon-dst := base-dir / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps' / appid + '.svg'

export RUSTFLAGS := env_var_or_default('RUSTFLAGS', '') + ' --cfg tokio_unstable '

# Default recipe which runs `just build-release`
default: build-release

# Runs `cargo clean`
clean: cargo::clean

# `cargo clean` and removes vendored dependencies
clean-dist: cargo::clean-dist
    mkdir -p .cargo
    cp data/cargo/config.toml .cargo/config.toml

# Compiles with debug profile
build-debug *args: (cargo::build-debug args)

# Compiles with release profile
build-release *args: (cargo::build-release args)

# Compiles release profile with vendored dependencies
build-vendored *args: (cargo::build-vendored args)

# Compiles and runs a standalone instance
run *args: (cargo::run args)

# Runs a clippy check
check *args: (cargo::check args)

# Runs a clippy check with JSON message format
check-json: (check '--message-format=json')

# Vendor dependencies locally
vendor: cargo::vendor

# Extracts vendored dependencies
vendor-extract: cargo::vendor-extract

# Build and run with tokio-console enabled
tokio-console: (build-release '--features console')
    env TOKIO_CONSOLE=1 {{bin-src}}

# Installs files
install:
    install -Dm0755 {{bin-src}} {{bin-dst}}
    install -Dm0644 {{ 'target' / 'xdgen' / desktop }} {{desktop-dst}}
    install -Dm0644 {{ 'target' / 'xdgen' / appdata }} {{appdata-dst}}
    install -Dm0644 {{ 'data' / 'icons' / appid + '.svg' }} {{icon-dst}}

# Uninstalls installed files
uninstall:
    rm {{bin-dst}} {{desktop-dst}} {{appdata-dst}} {{icon-dst}}
