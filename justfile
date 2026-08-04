clean:
	cargo clean
	rm -rf _build deps priv/ elixir/

run *args='-h':
	cargo run --release --bin getinbed -- {{args}}

bench:
	cargo bench

test:
	cargo test

test-elixir:
	mix deps.get && GETINBED_BUILD=1 mix test
