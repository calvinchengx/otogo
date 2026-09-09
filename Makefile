.PHONY: help install install-skills test smoke clean

help: ## Show the available targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) \
	  | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-10s\033[0m %s\n", $$1, $$2}'

install: ## Install otogo in editable mode with dev extras
	pip install -e ".[dev]"

install-skills: ## Link the otogo skills into a project's .claude/skills (DEST=path)
	@dest="$${DEST:-.claude/skills}"; mkdir -p "$$dest"; \
	for s in skills/*/; do \
	  name=$$(basename "$$s"); \
	  rm -rf "$$dest/$$name"; \
	  ln -s "$$(cd "$$s" && pwd)" "$$dest/$$name"; \
	  echo "  linked $$dest/$$name"; \
	done

test: ## Run the test suite
	pytest -q

smoke: ## Scaffold a throwaway loop and open a round against it
	@rm -rf .scratch/smoke && mkdir -p .scratch/smoke
	@cd .scratch/smoke && python3 -m otogo init . && python3 -m otogo status

clean: ## Remove build and test artifacts
	rm -rf .scratch dist build *.egg-info .pytest_cache
	find . -name __pycache__ -type d -prune -exec rm -rf {} +
