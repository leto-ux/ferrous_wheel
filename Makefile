.PHONY: install uninstall build

install:
	cargo install --path .

uninstall:
	cargo uninstall corroded_rsvp

build:
	cargo build --release
