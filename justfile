run *args='-h':
	cargo run --release --bin getinbed -- {{args}}

bench:
	cargo bench

test:
	cargo test

test-elixir:
	cd elixir && mix deps.get && GETINBED_BUILD=1 mix test
