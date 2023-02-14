default:
    just --justfile {{justfile()}} --choose --chooser=sk
compile:
    cargo build
install:
    cargo install --path .
update_rust:
    rustup update
update_lock:
    cargo update