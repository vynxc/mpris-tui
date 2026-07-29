.PHONY: build check media act

build:
	cargo build --release --locked

check:
	./scripts/check.sh

media:
	python3 tools/render_readme.py

act:
	act pull_request -W .github/workflows/ci.yml -j test

