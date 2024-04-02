#!/usr/env/bin bash

lint() {
  cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- --D warnings \
    -D unsafe_code \
    -D missing_copy_implmentations \
    -D trivial_casts \
    -D trivial_numeric_casts \
    -D unused_extern_crates \ 
    -D unused_import_crates \
    -D unused_import_braces \
    -D unused_qualifications \
    -D unreachable_pub
}

build() {
  cargo build --release
}

help() {
  echo "Usage: $(basename "$0") [OPTIONS
  Commands:
    lint    Run lints
    build   Build release binaries
    help    Show help
  "
}

if [[ $1 =~ ^(build|lint|help)$ ]]; then
  "$@"
else
  echo "Invalid command '$1'" >&2
  exit 1
fi
