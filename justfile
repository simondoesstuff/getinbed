run *args='-h':
	cargo run --release --bin getinbed -- {{args}}

bench:
	cargo bench

test:
	cargo test
