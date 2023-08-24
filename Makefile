all: 
	cargo build --release
install:
	strip ./target/release/l8ash
	[ `id -u` = 0 ]  && install -m 755 ./target/release/l8ash /usr/bin/ || install -m 755 ./target/release/l8ash $(PREFIX)/bin/
run:
	cargo run -- -v
test:
	cargo test
cov:
	cargo llvm-cov
