VERSION := `git rev-parse HEAD`
open := if os() == "macos" { "open" } else { "xdg-open" }

[private]
default:
  @just --list --unsorted

clippy:
  #rustup component add clippy --toolchain nightly
  cargo +nightly clippy --workspace
  cargo +nightly clippy --all-features

fmt:
  #rustup component add rustfmt --toolchain nightly
  rustfmt +nightly --edition 2021 $(find . -type f -iname *.rs)

doc:
  RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features --no-deps --open

deny:
  # might require rm Cargo.lock first to match CI
  cargo deny --workspace --all-features check bans licenses sources

# Unit tests
test:
  #!/usr/bin/env bash
  if rg "\`\`\`ignored"; then
    echo "ignored doctests are not allowed, use compile_fail or no_run"
    exit 1
  fi
  # no default features
  cargo test --workspace --lib --no-default-features
  # default features
  cargo test --workspace --lib --exclude kube-examples --exclude e2e
  # all features
  cargo test --workspace --lib --all-features --exclude kube-examples --exclude e2e
  cargo test --workspace --doc --all-features --exclude kube-examples --exclude e2e
  cargo test -p kube-examples --examples

# Integration tests (will modify your current context's cluster)
test-integration:
  kubectl delete pod -lapp=kube-rs-test > /dev/null
  cargo test --lib --workspace --exclude e2e --all-features -- --ignored
  # some examples are canonical tests
  cargo run -p kube-examples --example crd_derive
  cargo run -p kube-examples --example crd_api

coverage:
  # tarpaulin is currently broken for Rust 1.83, see #1657
  cargo +1.82.0 tarpaulin --out=Html --output-dir=.
  {{open}} tarpaulin-report.html

hack:
  time cargo hack check --feature-powerset --no-private -p kube \
    --skip=oauth,oidc \
    --group-features=socks5,http-proxy,gzip,ws \
    --group-features=admission,jsonpatch,derive \
    --group-features=rustls-tls,aws-lc-rs
  # Test groups features with minimal overlap that are grouped to reduce combinations.
  # Without any grouping this test takes an hour and has to test >11k combinations.
  # Skipped oauth and oidc, as these compile fails without a tls stack.

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
  k3d cluster create main --servers 1 --registry-create main --image rancher/k3s:v1.27.3-k3s1 \
    -p 10250:10250 --no-rollback \
    --k3s-arg "--disable=traefik,servicelb,metrics-server@server:*" \
    --k3s-arg '--kubelet-arg=eviction-hard=imagefs.available<1%,nodefs.available<1%@agent:*' \
    --k3s-arg '--kubelet-arg=eviction-minimum-reclaim=imagefs.available=1%,nodefs.available=1%@agent:*' \
    --k3s-arg '--kube-apiserver-arg=feature-gates=WatchList=true@server:*'

## RELEASE RELATED

# Bump the msrv of kube; "just bump-msrv 1.60.0"
bump-msrv msrv:
  #!/usr/bin/env bash
  fullmsrv="{{msrv}}" # need a temporary var for this
  shortmsrv="${fullmsrv::-2}" # badge can use a short display version
  badge="[![Rust ${shortmsrv}](https://img.shields.io/badge/MSRV-${shortmsrv}-dea584.svg)](https://github.com/rust-lang/rust/releases/tag/{{msrv}})"
  sd "rust-version = \".*\"" "rust-version = \"{{msrv}}\"" Cargo.toml
  sd "^.+badge/MSRV.+$" "${badge}" README.md
  sd "rust:.*-bullseye" "rust:{{msrv}}-bullseye" .devcontainer/Dockerfile

# Sets the Kubernetes feature version from latest k8s-openapi.
bump-k8s:
  #!/usr/bin/env bash
  earliest=$(cargo info k8s-openapi --color=never 2> /dev/null |grep earliest | awk -F'[][]' '{print $2}')
  latest=$(cargo info k8s-openapi --color=never 2> /dev/null |grep latest | awk -F'[][]' '{print $2}')
  # pin mk8sv to k8s-openapi earliest
  min_feat="${earliest::-2}${earliest:3}"
  min_dots="${min_feat/_/.}"
  echo "Setting MK8SV to $min_dots using feature $min_feat"
  # workflow pins for k3s (any line with key/array suffixed by # MK8SV)
  sd "(.*)([\:\-]{1}) .* # MK8SV$" "\$1\$2 \"${min_dots}\" # MK8SV" .github/workflows/*.yml
  # bump mk8sv badge
  badge="[![Tested against Kubernetes ${min_dots} and above](https://img.shields.io/badge/MK8SV-${min_dots}-326ce5.svg)](https://kube.rs/kubernetes-version)"
  sd "^.+badge/MK8SV.+$" "${badge}" README.md
  echo "remember to bump kubernetes-version.md in kube-rs/website"
