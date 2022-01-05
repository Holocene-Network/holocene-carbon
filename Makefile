MAKEFLAGS		+= --silent
PROGRAM			:= holocene_carbon
BUILD_BASE		:= target/ink
BUILD_DEBUG		:= build/debug
BUILD_RELEASE	:= build/release

.PHONY : debug clean check clippy fmt upgrade test all

all: | prepare release

test:
	cargo contract test

prepare:
	mkdir -p ${BUILD_DEBUG}
	mkdir -p ${BUILD_RELEASE}

check:
	cargo contract check

clippy:
	cargo clippy

upgrade:
	cargo update
	cargo upgrade

fmt:
	cargo fmt

clean:
	rm -rf target
	rm -rf build

debug: | prepare
	cargo contract build
	cp ${BUILD_BASE}/${PROGRAM}.wasm ${BUILD_DEBUG}/${PROGRAM}.wasm
	cp ${BUILD_BASE}/${PROGRAM}.contract ${BUILD_DEBUG}/${PROGRAM}.contract
	cp ${BUILD_BASE}/metadata.json ${BUILD_DEBUG}/metadata.json

release: | prepare
	cargo contract build --release
	cp ${BUILD_BASE}/${PROGRAM}.wasm ${BUILD_RELEASE}/${PROGRAM}.wasm
	cp ${BUILD_BASE}/${PROGRAM}.contract ${BUILD_RELEASE}/${PROGRAM}.contract
	cp ${BUILD_BASE}/metadata.json ${BUILD_RELEASE}/metadata.json
