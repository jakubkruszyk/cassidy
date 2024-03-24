single:
	./target/debug/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 1 --seed 1

iter:
	./target/debug/cassidy --with-config ./tests/references/default_config.toml --duration 24 --iterations 10 --seed 1 --show-partial-results
