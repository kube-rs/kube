VERSION=$(shell grep version Cargo.toml | awk -F"\"" '{print $$2}' | head -n 1)

clippy:
	touch src/lib.rs
	cargo clippy -p kube -- #--allow clippy::or_fun_call --allow clippy::redundant_pattern_matching

doc:
	cargo doc --lib
	xdg-open target/doc/kube/index.html

fmt:
	#rustup component add rustfmt --toolchain nightly
	cargo +nightly fmt

test:
	cargo test --all-features

readme:
	rustdoc README.md --test --edition=2018

.PHONY: doc build fmt clippy test readme
