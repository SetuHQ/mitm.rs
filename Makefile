
default: watch

build:
	@cargo build --features "yaml"

run:
	@cargo run

watch:
	nodemon -e rs --watch ./ --signal SIGKILL --exec "clear && cargo build || exit 1"
