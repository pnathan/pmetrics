PGPASSWORD ?= aargh
PGHOST     ?= localhost
PGUSER     ?= postgres
PGDATABASE ?= postgres
PGPORT     ?= 5432

export PGPASSWORD PGHOST PGUSER PGDATABASE PGPORT

.PHONY: coverage

coverage:
	cargo llvm-cov --all-features --workspace --summary-only
