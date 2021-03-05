
default: watch

build:
	@cargo build --features "yaml"

run:
	@RUST_BACKTRACE=1 cargo run

watch:
	nodemon -e rs --watch ./ --signal SIGKILL --exec "clear && cargo build || exit 1"

watch-run:
	nodemon -e rs --watch ./ --signal SIGKILL --exec "clear && RUST_BACKTRACE=1 cargo run || exit 1"
