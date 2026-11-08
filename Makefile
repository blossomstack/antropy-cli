# The wire types are generated from protocol/antropy.fl, a copy of the
# server's schema. Pin the generator version: an unpinned bump would redden
# unrelated pull requests.
FLUORITE_VERSION ?= 0.9.0
FLUORITE ?= npx --yes @blossomstack/fluorite-cli@$(FLUORITE_VERSION)

SCHEMA   := protocol/antropy.fl
RUST_OUT := crates/antropy-protocol/src/generated
DERIVES  := Debug,Clone,Default,PartialEq,serde::Serialize,serde::Deserialize

.PHONY: types types-check check lint test all

types:
	$(FLUORITE) clean --output $(RUST_OUT)
	$(FLUORITE) rust --inputs $(SCHEMA) --output $(RUST_OUT) \
		--single-file --visibility public --derives "$(DERIVES)"

types-check: types
	@git diff --exit-code -- $(RUST_OUT) || \
		{ echo "Generated types are stale. Run 'make types' and commit the result."; exit 1; }
	@test -z "`git status --porcelain -- $(RUST_OUT)`" || \
		{ echo "Generation added or removed files. Run 'make types' and commit the result."; exit 1; }

check:
	cargo fmt --all -- --check

lint:
	cargo clippy --locked --all-targets -- -D warnings

test:
	cargo test --locked --workspace

all: check lint test types-check
