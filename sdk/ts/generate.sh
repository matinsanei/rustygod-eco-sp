#!/usr/bin/env bash
# Regenerate TypeScript gRPC clients from the proto source of truth.
# Run from sdk/ts:  npm run gen
set -euo pipefail
cd "$(dirname "$0")"

PROTO_DIR="../../crates/proto/proto"
PROTOC="./node_modules/.bin/grpc_tools_node_protoc"
TS_PLUGIN="./node_modules/.bin/protoc-gen-ts_proto"

"$PROTOC" \
  --plugin="protoc-gen-ts=${TS_PLUGIN}" \
  --ts_out="./src" \
  --ts_opt="outputServices=grpc-js,env=node,esModuleInterop=true,outputJsonMethods=false,importSuffix=.js" \
  -I "$PROTO_DIR" \
  common.proto product.proto checkout.proto order.proto commerce.proto \
  ai.proto payment.proto webhook.proto auth.proto giftcard.proto \
  draft.proto invoice.proto plugin.proto

# commerce.ts declares getChannel, which collides with grpc-js's built-in
# Client.getChannel() — a known ts-proto/grpc-js edge, not a proto bug.
# (Renaming the RPC would break the wire contract.)
sed -i '1i // @ts-nocheck: getChannel collides with grpc-js Client.getChannel' ./src/commerce.ts

echo "generated $(find src -name '*.ts' | wc -l) TS modules"
