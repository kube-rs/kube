VERSION=$(shell git rev-parse HEAD)

clippy:
	#rustup component add clippy --toolchain nightly
	touch kube/src/lib.rs
	cd kube && cargo +nightly clippy --no-default-features --features=rustls-tls
	cd kube && cargo +nightly clippy --no-default-features --features=rustls-tls --examples
	cd kube-derive && cargo +nightly clippy

fmt:
	#rustup component add rustfmt --toolchain nightly
	rustfmt +nightly --edition 2021 $$(find . -type f -iname *.rs)

doc:
	RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --lib --workspace --features=derive,ws,oauth,jsonpatch,client,derive,runtime,admission,k8s-openapi/v1_22 --open

test:
	cargo test --lib --all
	cargo test --doc --all
	cargo test -p kube-examples --examples
	cargo test -p kube --lib --no-default-features --features=rustls-tls,ws,oauth
	cargo test -p kube --lib --no-default-features --features=native-tls,ws,oauth
	cargo test -p kube --lib --no-default-features
	cargo test -p kube-examples --example crd_api --no-default-features --features=deprecated,kubederive,native-tls

test-integration:
	cargo test --lib --all -- --ignored # also run tests that fail on github actions
	cargo test -p kube --lib --features=derive,runtime -- --ignored
	cargo test -p kube-client --lib --features=rustls-tls,ws -- --ignored
	cargo run -p kube-examples --example crd_derive
	cargo run -p kube-examples --example crd_api

readme:
	rustdoc README.md --test --edition=2021

e2e: dapp
	ls -lah e2e/
	docker build -t clux/kube-dapp:$(VERSION) e2e/
	k3d image import clux/kube-dapp:$(VERSION) --cluster main
	sed -i 's/latest/$(VERSION)/g' e2e/deployment.yaml
	kubectl apply -f e2e/deployment.yaml
	sed -i 's/$(VERSION)/latest/g' e2e/deployment.yaml
	kubectl get all -n apps
	kubectl describe jobs/dapp -n apps
	kubectl wait --for=condition=complete job/dapp -n apps --timeout=50s || kubectl logs -f job/dapp -n apps
	kubectl get all -n apps
	kubectl wait --for=condition=complete job/dapp -n apps --timeout=10s || kubectl get pods -n apps | grep dapp | grep Completed

dapp:
	docker run \
		-v cargo-cache:/root/.cargo/registry \
		-v "$$PWD:/volume" -w /volume \
		--rm -it clux/muslrust:stable cargo build --release -p e2e
	cp target/x86_64-unknown-linux-musl/release/dapp e2e/dapp
	chmod +x e2e/dapp

k3d:
	k3d cluster create --servers 1 --agents 1 main \
		--k3s-agent-arg '--kubelet-arg=eviction-hard=imagefs.available<1%,nodefs.available<1%' \
		--k3s-agent-arg '--kubelet-arg=eviction-minimum-reclaim=imagefs.available=1%,nodefs.available=1%'

.PHONY: doc build fmt clippy test readme k3d e2e
