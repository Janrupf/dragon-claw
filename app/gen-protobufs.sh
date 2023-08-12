#!/usr/bin/env bash
set -e

gen_plugin=$(command -v protoc-gen-dart)
if [[ -z "$gen_plugin" ]]; then
  gen_plugin="$(realpath ~/.pub-cache/bin/protoc-gen-dart)"
fi

echo "Using protoc-gen-dart at $gen_plugin"

exec protoc \
  --dart_out=grpc:lib/generated \
  --plugin="$gen_plugin" \
  -I../proto \
  ../proto/google/protobuf/empty.proto \
  ../proto/*.proto
