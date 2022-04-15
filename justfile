rootdir := ''
prefix := '/usr'
clean := '0'
debug := '0'
vendor := '0'
target := if debug == '1' { 'debug' } else { 'release' }
vendor_args := if vendor == '1' { '--frozen --offline' } else { '' }
debug_args := if debug == '1' { '' } else { '--release' }
feature_args :=  "--no-default-features"
cargo_args := vendor_args + ' ' + debug_args + ' ' + feature_args

id := 'com.System76.CosmicLauncher'

sharedir := rootdir + prefix + '/share'
iconsdir := sharedir + '/icons/hicolor/scalable/apps'
bindir := rootdir + prefix + '/bin'

all: _extract_vendor _compile_gresource
    cargo build {{cargo_args}}

# Installs files into the system
install:
    install -Dm0644 data/icons/{{id}}-symbolic.svg {{iconsdir}}/{{id}}-symbolic.svg
    install -Dm0644 data/icons/{{id}}.Devel.svg {{iconsdir}}/{{id}}.Devel.svg
    install -Dm0644 data/icons/{{id}}.svg {{iconsdir}}/{{id}}.svg
    install -Dm0644 data/{{id}}.desktop {{sharedir}}/applications/{{id}}.desktop
    install -Dm0644 target/compiled.gresource {{sharedir}}/{{id}}/compiled.gresource
    install -Dm04755 target/release/cosmic-launcher {{bindir}}/cosmic-launcher

# Extracts vendored dependencies if vendor=1
_extract_vendor:
    #!/usr/bin/env sh
    if test {{vendor}} = 1; then
        rm -rf vendor; tar pxf vendor.tar
    fi

# Compiles the gresources file
_compile_gresource:
    mkdir -p target/
    glib-compile-resources --sourcedir=data/resources \
        --target=target/compiled.gresource \
        data/resources/resources.gresource.xml