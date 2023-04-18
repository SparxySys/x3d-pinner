debug ?=

ifdef debug
  release :=
  target :=debug
  extension :=debug
else
  release :=--release
  target :=release
  extension :=
endif

ifeq ($(PREFIX),)
    PREFIX := /usr/local
endif

build:
	cargo build $(release)

install:
	install -d $(DESTDIR)$(PREFIX)/bin/
	install -m 755 target/$(target)/x3d-pinner $(DESTDIR)$(PREFIX)/bin/

all: build install

clean:
	cargo clean
 
help:
	@echo "usage: make [debug=1]"
