VERSION := `git rev-parse HEAD`

default:
  @just --list --unsorted --color=always | rg -v "    default"

clippy:
  #rustup component add clippy --toolchain nightly
  cargo +nightly clippy --workspace
  cargo +nightly clippy --no-default-features --features=rustls-tls

fmt:
  #rustup component add rustfmt --toolchain nightly
  rustfmt +nightly --edition 2021 $(find . -type f -iname *.rs)

doc:
  RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --lib --workspace --features=derive,ws,oauth,jsonpatch,client,derive,runtime,admission,k8s-openapi/v1_25 --open

# Unit tests
test:
  cargo test --lib --all
  cargo test --doc --all
  cargo test -p kube-examples --examples
  cargo test -p kube --lib --no-default-features --features=rustls-tls,ws,oauth
  cargo test -p kube --lib --no-default-features --features=openssl-tls,ws,oauth
  cargo test -p kube --lib --no-default-features

test-integration:
  kubectl delete pod -lapp=kube-rs-test
  cargo test --lib --all -- --ignored # also run tests that fail on github actions
  cargo test -p kube --lib --features=derive,runtime -- --ignored
  cargo test -p kube-client --lib --features=rustls-tls,ws -- --ignored
  cargo run -p kube-examples --example crd_derive
  cargo run -p kube-examples --example crd_api

coverage:
  cargo tarpaulin --out=Html --output-dir=.
  #xdg-open tarpaulin-report.html

deny:
  # might require rm Cargo.lock first to match CI
  cargo deny --workspace --all-features check bans licenses sources

readme:
  rustdoc README.md --test --edition=2021

e2e: (e2e-mink8s) (e2e-incluster "rustls,latest")

e2e-mink8s:
  cargo run -p e2e --bin boot --features=openssl,latest
  cargo run -p e2e --bin boot --features=openssl,mk8sv

  #cargo run -p e2e --bin boot --features=rustls,latest
  #cargo run -p e2e --bin boot --features=rustls,mk8sv

e2e-incluster features:
  just e2e-job-musl {{features}}
  docker build -t clux/kube-e2e:{{VERSION}} e2e/
  k3d image import clux/kube-e2e:{{VERSION}} --cluster main
  sed -i 's/latest/{{VERSION}}/g' e2e/deployment.yaml
  kubectl apply -f e2e/deployment.yaml
  sed -i 's/{{VERSION}}/latest/g' e2e/deployment.yaml
  kubectl get all -n apps
  kubectl describe jobs/e2e -n apps
  kubectl wait --for=condition=complete job/e2e -n apps --timeout=50s || kubectl logs -f job/e2e -n apps
  kubectl get all -n apps
  kubectl wait --for=condition=complete job/e2e -n apps --timeout=10s || kubectl get pods -n apps | grep e2e | grep Completed
e2e-job-musl features:
  #!/usr/bin/env bash
  docker run \
    -v cargo-cache:/root/.cargo/registry \
    -v "$PWD:/volume" -w /volume \
    --rm -it clux/muslrust:stable cargo build --release --features={{features}} -p e2e
  cp target/x86_64-unknown-linux-musl/release/job e2e/job
  chmod +x e2e/job

k3d:
  k3d cluster create main --servers 1 --registry-create main \
    --no-lb --no-rollback \
    --k3s-arg "--disable=traefik,servicelb,metrics-server@server:*" \
    --k3s-arg '--kubelet-arg=eviction-hard=imagefs.available<1%,nodefs.available<1%@agent:*' \
    --k3s-arg '--kubelet-arg=eviction-minimum-reclaim=imagefs.available=1%,nodefs.available=1%@agent:*'

## RELEASE RELATED

# Bump the msrv of kube; "just bump-msrv 1.60.0"
bump-msrv msrv:
  #!/usr/bin/env bash
  # TODO: warn if not msrv+2 not found
  oldmsrv="$(rg "rust-version = \"(.*)\"" -r '$1' kube/Cargo.toml)"
  fastmod -m -d . --extensions toml "rust-version = \"$oldmsrv\"" "rust-version = \"{{msrv}}\""
  # sanity
  if [[ $(cat ./*/Cargo.toml | grep "rust-version" | uniq | wc -l) -gt 1 ]]; then
    echo "inconsistent rust-version keys set in various kube-crates:"
    rg "rust-version" ./*/Cargo.toml
    exit 1
  fi
  fullmsrv="{{msrv}}"
  shortmsrv="${fullmsrv::-2}" # badge can use a short display version
  badge="[![Rust ${shortmsrv}](https://img.shields.io/badge/MSRV-${shortmsrv}-dea584.svg)](https://github.com/rust-lang/rust/releases/tag/{{msrv}})"
  sd "^.+badge/MSRV.+$" "${badge}" README.md
  sd "${oldmsrv}" "{{msrv}}" .devcontainer/Dockerfile
  cargo msrv

# Increment the Kubernetes feature version from k8s-openapi for tests; "just bump-k8s"
bump-k8s:
  #!/usr/bin/env bash
  current=$(cargo tree --format "{f}" -i k8s-openapi | head -n 1)
  next=${current::-2}$((${current:3} + 1))
  fastmod -m -d . -e toml "$current" "$next"
  fastmod -m "$current" "$next" -- README.md
  fastmod -m "$current" "$next" -- justfile
  # bumping supported version also bumps our mk8sv
  mk8svnew=${current::-2}$((${current:3} - 4))
  mk8svold=${current::-2}$((${current:3} - 5))
  fastmod -m -d e2e -e toml "$mk8svold" "$mk8svnew"
  fastmod -m -d .github/workflows -e yml "${mk8svold/_/\.}" "${mk8svnew/_/.}"
  # bump mk8sv badge
  badge="[![Tested against Kubernetes ${mk8svnew} and above](https://img.shields.io/badge/MK8SV-${mk8svnew}-326ce5.svg)](https://kube.rs/kubernetes-version)"
  sd "^.+badge/MK8SV.+$" "${badge}" README.md

# mode: makefile
# End:
# vim: set ft=make :
