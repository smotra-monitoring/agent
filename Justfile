all: test generate-omg

help:
    @just --list

generate-omg:
    omg --input api/openapi/api/spec.yaml --output src/openapi/omg/generated/

test:
    cargo test
