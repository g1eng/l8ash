all: 
	cargo build --release
install:
	strip ./target/release/leash
	[ `id -u` = 0 ]  && install -m 755 ./target/release/leash /usr/bin/ || install -m 755 ./target/release/leash $(PREFIX)/bin/
run:
	cargo run -- -v
test:
	cargo test
cov:
	cargo llvm-cov
