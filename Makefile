
default: watch

build: ## Build the server
	@cargo build

run: ## Run the server
	@RUST_BACKTRACE=1 cargo run

watch: ## Build, watch for changes and restart
	nodemon -e rs --watch ./ --signal SIGKILL --exec "clear && cargo build || exit 1"

watch-run: ## Build and run, watch for changes and restart
	nodemon -e rs --watch ./ --signal SIGKILL --exec "clear && RUST_BACKTRACE=1 cargo run || exit 1"

release: ## Create a release binary `mitm.rs`
	@cargo build --release
	@mv ./target/release/mitm ./mitm.rs

publish: ## Publish into crates.io
	@cargo login ${TOKEN}
	@cargo publish

.PHONY: help
help: ## Dislay this help
	@IFS=$$'\n'; for line in `grep -h -E '^[a-zA-Z_#-]+:?.*?## .*$$' $(MAKEFILE_LIST)`; do if [ "$${line:0:2}" = "##" ]; then \
	echo $$line | awk 'BEGIN {FS = "## "}; {printf "\n\033[33m%s\033[0m\n", $$2}'; else \
	echo $$line | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'; fi; \
	done; unset IFS;
