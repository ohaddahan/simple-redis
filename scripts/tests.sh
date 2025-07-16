#!/usr/bin/env bash
export _PWD="$(pwd)"
export ROOT="$(git rev-parse --show-toplevel)"
source "${ROOT}/scripts/setup.sh"
source "${ROOT}/scripts/init_db.sh"
export RUST_BACKTRACE=1
export REDIS_URL="redis://127.0.0.1:6379"
cargo test -- --nocapture