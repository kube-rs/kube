VERSION=$(shell git rev-parse HEAD)

clippy:
	#rustup component add clippy --toolchain nightly
	touch kube/src/lib.rs
	cd kube && cargo +nightly clippy --no-default-features --features=rustls-tls
	cd kube && cargo +nightly clippy --no-default-features --features=rustls-tls --examples -- --allow clippy::or_fun_call --allow clippy::blacklisted_name
	cd kube-derive && cargo +nightly clippy

fmt:
	#rustup component add rustfmt --toolchain nightly
	cargo +nightly fmt

doc:
	cargo +nightly doc --lib
	xdg-open target/doc/kube/index.html

test:
	cargo test --all
	cargo test --lib --all -- --ignored # also run tests that fail on circleci
	cd kube && cargo test --lib --features=rustls-tls --no-default-features
	cd kube && cargo test --lib --features=derive

readme:
	rustdoc README.md --test --edition=2018

minikube-create:
	sudo rm -rf /tmp/juju-mk* /tmp/minikube*
	minikube start --driver=docker \
		--kubernetes-version v1.20.2 \
		--extra-config kubeadm.ignore-preflight-errors=SystemVerification

minikube:
	kubectl config set-context --cluster=minikube --user=minikube --namespace=apps minikube
	kubectl create namespace apps

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
