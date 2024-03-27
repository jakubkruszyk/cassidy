single_d:
	cargo build
	./target/debug/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 1 --seed 1

single:
	cargo build --release
	./target/release/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 1 --seed 1

iter:
	cargo build --release
	./target/debug/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 10 --seed 1 --show-partial-results
