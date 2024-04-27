single_d:
	cargo build
	./target/debug/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 1 --seed 1

single:
	cargo build --release
	./target/release/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 1 --seed 1

iter:
	cargo build --release
	./target/debug/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 10 --seed 1 --show-partial-results

single_sleep:
	cargo build --release
	./target/release/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 1 --seed 1 --enable-sleep --log-wave

walk_lambda:
	cargo build --release
	./target/release/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 1 --seed 1 --walk-over ./tests/references/walk_over.toml
