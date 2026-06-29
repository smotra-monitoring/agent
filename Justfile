all: test generate-omg

help:
    @just --list

generate-omg:
    omg --input api/openapi/api/spec.yaml --output src/openapi/omg/generated/
    grep -qF 'use chrono::{DateTime, Utc};' src/openapi/omg/generated/models.rs || \
        sed -i 's/^use serde::{Deserialize, Serialize};/use chrono::{DateTime, Utc};\nuse serde::{Deserialize, Serialize};/' src/openapi/omg/generated/models.rs

test:
    cargo test
