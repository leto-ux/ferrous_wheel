.PHONY: install uninstall build

install:
	cargo install --path .

uninstall:
	cargo uninstall ferrous_wheel

build:
	cargo build --release
