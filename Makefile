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
	cargo test --lib
	cargo test --doc
	cargo test --lib -- --ignored # also run tests that fail on circleci
	cargo test --example crd_api crd_reflector
	cargo test -j4
	cd kube && cargo test --lib --features=rustls-tls --no-default-features

readme:
	rustdoc README.md --test --edition=2018

bump-minor:
	./release.sh minor

bump-patch:
	./release.sh patch

publish:
	./release.sh publish

minikube-create:
	sudo rm -rf /tmp/juju-mk* /tmp/minikube*
	sudo -E minikube start --driver=none --kubernetes-version v1.17.4 --extra-config kubeadm.ignore-preflight-errors=SystemVerification

minikube:
	kubectl config set-context --cluster=minikube --user=minikube --namespace=apps minikube
	kubectl create namespace apps

kind-create:
	kind create cluster --name kube

kind:
	kubectl config set-context --cluster=kind-kube --user=kind-kube --namespace=apps kind-kube
	kubectl config use-context kind-kube

kind-delete:
	kind delete clusters kind-kube

.PHONY: doc build fmt clippy test readme minikube kind
