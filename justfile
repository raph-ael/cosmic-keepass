name := 'cosmic-keepass'
appid := 'io.github.cosmic-keepass'

rootdir := ''
prefix := '/usr'

base-dir := absolute_path(clean(rootdir / prefix))

bin-src := 'target' / 'release' / name
bin-dst := base-dir / 'bin' / name

desktop := appid + '.desktop'
desktop-src := 'resources' / 'app.desktop'
desktop-dst := clean(rootdir / prefix) / 'share' / 'applications' / desktop

icons-src := 'resources' / 'icons' / 'hicolor'
icons-dst := clean(rootdir / prefix) / 'share' / 'icons' / 'hicolor'

icon-svg-src := icons-src / 'scalable' / 'apps' / 'icon.svg'
icon-svg-dst := icons-dst / 'scalable' / 'apps' / appid + '.svg'

default: build-release

clean:
    cargo clean

build-debug *args:
    cargo build {{args}}

build-release *args: (build-debug '--release' args)

check *args:
    cargo clippy --all-features {{args}} -- -W clippy::pedantic

run *args:
    env RUST_BACKTRACE=full cargo run --release {{args}}

install:
    install -Dm0755 {{bin-src}} {{bin-dst}}
    install -Dm0644 {{desktop-src}} {{desktop-dst}}
    install -Dm0644 {{icon-svg-src}} {{icon-svg-dst}}

uninstall:
    rm {{bin-dst}} {{desktop-dst}} {{icon-svg-dst}}
