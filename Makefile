VERSION=$(shell git rev-parse HEAD)

clippy:
	#rustup component add clippy --toolchain nightly
	touch kube/src/lib.rs
	cd kube && cargo +nightly clippy --no-default-features --features=rustls-tls
	cd kube && cargo +nightly clippy --no-default-features --features=rustls-tls --examples
	cd kube-derive && cargo +nightly clippy

fmt:
	#rustup component add rustfmt --toolchain nightly
	rustfmt +nightly --edition 2018 $$(find . -type f -iname *.rs)

doc:
	RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --lib --workspace --features=derive,ws,oauth,jsonpatch,client,derive,runtime,admission --open

test:
	cargo test --all
	cargo test --lib --all -- --ignored # also run tests that fail on circleci
	cd kube && cargo test --lib --features=rustls-tls,client --no-default-features
	cd kube && cargo test --lib --no-default-features
	cd kube && cargo test --lib --features=derive

readme:
	rustdoc README.md --test --edition=2018

kind-create:
	kind create cluster

kind:
	kubectl config set-context --cluster=kind-kind --user=kind-kind --namespace=apps kind-kind
	kubectl config use-context kind-kind

kind-delete:
	kind delete clusters kind-kind

# local integration tests:
dapp:
	docker run \
		-v cargo-cache:/root/.cargo/registry \
		-v "$$PWD:/volume" -w /volume \
		--rm -it clux/muslrust:stable cargo build --release -p tests
	cp target/x86_64-unknown-linux-musl/release/dapp tests/dapp
	chmod +x tests/dapp
integration-test: dapp
	docker build -t clux/kube-dapp:latest tests/
	kubectl apply -f tests/deployment.yaml
	kubectl rollout status deploy/dapp -n apps
	kubectl status deploy/dapp -n apps
	kubectl logs -f -n apps deploy/dapp
	kubectl get pods -n apps | grep dapp | grep Completed
	kubectl get pods -n apps | grep empty-job | grep Completed

# for ci (has dapp built)
integration-ci:
	ls -lah tests/
	docker build -t clux/kube-dapp:$(VERSION) tests/
	docker push clux/kube-dapp:$(VERSION) || true
	./kind load docker-image clux/kube-dapp:$(VERSION)
	sed -i 's/latest/$(VERSION)/g' tests/deployment.yaml

# to debug ci...
integration-pull:
	docker pull clux/kube-dapp:$(VERSION)
	sed -i 's/latest/$(VERSION)/g' tests/deployment.yaml
	kubectl apply -f tests/deployment.yaml

.PHONY: doc build fmt clippy test readme minikube kind
