VERSION=$(shell git rev-parse HEAD)

clippy:
	#rustup component add clippy --toolchain nightly
	cargo +nightly clippy --workspace
	cargo +nightly clippy --no-default-features --features=rustls-tls

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
	kubectl delete pod -lapp=kube-rs-test
	cargo test --lib --all -- --ignored # also run tests that fail on github actions
	cargo test -p kube --lib --features=derive,runtime -- --ignored
	cargo test -p kube-client --lib --features=rustls-tls,ws -- --ignored
	cargo run -p kube-examples --example crd_derive
	cargo run -p kube-examples --example crd_api
	cargo run -p kube-examples --example node_cordon

coverage:
	cargo tarpaulin --out=Html --output-dir=.
	#xdg-open tarpaulin-report.html

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
	k3d cluster create main --servers 1 --agents 1 --registry-create main \
		--k3s-arg "--no-deploy=traefik@server:*" \
		--k3s-arg '--kubelet-arg=eviction-hard=imagefs.available<1%,nodefs.available<1%@agent:*' \
		--k3s-arg '--kubelet-arg=eviction-minimum-reclaim=imagefs.available=1%,nodefs.available=1%@agent:*'

.PHONY: doc build fmt clippy test readme k3d e2e
