#!/bin/sh
podman run \
       -it \
       --rm \
       -v ./target/debug/twardyece-manager:/usr/bin/twardyece-manager:ro \
       -v ./test/config.yaml:/etc/redfish/config.yaml:ro \
       -p 3000:3000 \
       --entrypoint=/usr/bin/twardyece-manager \
       -e 'RUST_LOG=tower_http=debug' \
       seuss-test:latest \
       --config /etc/redfish/config.yaml
