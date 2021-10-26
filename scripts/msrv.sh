#!/bin/bash
set -euo pipefail

main() {
    local -r msrv="$(cargo msrv --output-format=json | jq -r 'select(.reason == "msrv-complete") | .msrv')"
    local -r badge="[![Rust ${msrv::-2}](https://img.shields.io/badge/MSRV-${msrv::-2}-dea584.svg)](https://github.com/rust-lang/rust/releases/tag/${msrv})"
    sd "^.+badge/MSRV.+$" "${badge}" README.md
}


# helper script to be run by users when bumping dependencies
#
# This script determines our msrv using cargo-msrv, and then
# edits the badge in the README using `sd`.
#
# shellcheck disable=SC2068
main $@
