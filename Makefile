.PHONY: help build install install-skills test fmt lint smoke clean

help: ## Show the available targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
	  | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build the release binary
	cargo build --release
	@ls -lh target/release/otogo | awk '{print "  binary:", $$5}'

install: ## Install otogo onto PATH
	cargo install --path .

install-skills: ## Link the otogo skills into a project's .claude/skills (DEST=path)
	@dest="$${DEST:-.claude/skills}"; mkdir -p "$$dest"; \
	for s in skills/*/; do \
	  name=$$(basename "$$s"); \
	  rm -rf "$$dest/$$name"; \
	  ln -s "$$(cd "$$s" && pwd)" "$$dest/$$name"; \
	  echo "  linked $$dest/$$name"; \
	done

test: ## Run the test suite
	cargo test

fmt: ## Format
	cargo fmt

lint: ## Clippy, warnings as errors
	cargo clippy --all-targets -- -D warnings

smoke: ## Scaffold a throwaway loop and open a round against it
	@rm -rf scratch/smoke && mkdir -p scratch/smoke
	@cd scratch/smoke && $(CURDIR)/target/release/otogo init . && $(CURDIR)/target/release/otogo status

clean: ## Remove build and test artifacts
	cargo clean
	rm -rf scratch
