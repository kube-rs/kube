VERSION=$(shell grep version Cargo.toml | awk -F"\"" '{print $$2}' | head -n 1)

clippy:
	touch src/lib.rs
	cargo clippy -p kube -- #--allow clippy::or_fun_call --allow clippy::redundant_pattern_matching

doc:
	cargo doc --lib
	xdg-open target/doc/kube/index.html

push-docs:
	cargo doc --lib -p kube
	echo "<meta http-equiv=refresh content=0;url=kube/index.html>" > target/doc/index.html
	ghp-import -n target/doc
	git push -qf "git@github.com:clux/kube-rs.git" gh-pages

.PHONY: doc build
