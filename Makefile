
default: watch

build:
	@cargo build

run:
	@RUST_BACKTRACE=1 cargo run

watch:
	nodemon -e rs --watch ./ --signal SIGKILL --exec "clear && cargo build || exit 1"

watch-run:
	nodemon -e rs --watch ./ --signal SIGKILL --exec "clear && RUST_BACKTRACE=1 cargo run || exit 1"

release:
	@cargo build --release
	@mv ./target/release/mitm ./mitm.rs

publish:
	@cargo login ${TOKEN}
	@cargo publish
