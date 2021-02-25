
default: watch

build:
	@cargo build

run:
	@cargo run

watch:
	nodemon -e rs --watch ./ --signal SIGKILL --exec "cargo build || exit 1"
