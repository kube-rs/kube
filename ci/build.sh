set -euo pipefail

if [ ! -d ~/.cargo/bin ]; then
	mkdir -p ~/.cargo/bin
	curl -Lo ~/.cargo/bin/rustup 'https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init'
	chmod +x ~/.cargo/bin/rustup
	ln -s ~/.cargo/bin/rustup ~/.cargo/bin/cargo
	ln -s ~/.cargo/bin/rustup ~/.cargo/bin/cargo-clippy
	ln -s ~/.cargo/bin/rustup ~/.cargo/bin/rustc
	ln -s ~/.cargo/bin/rustup ~/.cargo/bin/rustdoc
	export PATH="$PATH:$(realpath ~/.cargo/bin)"
fi

rustup install stable
rustup default stable

# Saves a few seconds for large crates
export CARGO_INCREMENTAL=0

cargo build --all-features
cargo test --all-features --no-run

export RUST_BACKTRACE=full
cargo test --all-features

cargo doc --no-deps --features --all-features
