#!/usr/bin/env bash
set -e

gen_plugin="$(realpath ~/.pub-cache/bin/protoc-gen-dart)"
exec protoc \
  --dart_out=grpc:lib/generated \
  --plugin="$gen_plugin" \
  -I../proto \
  ../proto/google/protobuf/empty.proto \
  ../proto/*.proto
