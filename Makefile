VERSION=$(shell grep version Cargo.toml | awk -F"\"" '{print $$2}' | head -n 1)

clippy:
	#rustup component add clippy --toolchain nightly
	touch kube/src/lib.rs
	cd kube && cargo +nightly clippy --no-default-features --features=rustls-tls
	cd kube && cargo +nightly clippy --no-default-features --features=rustls-tls --examples -- --allow clippy::or_fun_call

fmt:
	#rustup component add rustfmt --toolchain nightly
	cargo +nightly fmt

doc:
	cargo doc --lib
	xdg-open target/doc/kube/index.html

test:
	cargo test --lib
	cargo test --example crd_api crd_reflector
	cargo test -j4
	cd kube && cargo test --lib --features=rustls-tls --no-default-features

readme:
	rustdoc README.md --test --edition=2018

.PHONY: doc build fmt clippy test readme
