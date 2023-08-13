
default: debug

debug:
    cargo run

devsetup:
    cp dev/hooks/* .git/hooks

fmt:
    cargo +nightly fmt --all

lint:
    cargo clippy -- -W clippy::unwrap_used -W clippy::cargo

install:
    cargo install --profile=release --path .
