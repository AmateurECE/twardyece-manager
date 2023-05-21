#!/bin/sh
podman run \
       -it \
       --rm \
       -v ./target/debug/$1:/usr/bin/$1:ro \
       -v ./test/config.yaml:/etc/redfish/config.yaml:ro \
       -v ./test/manager-key.pem:/etc/redfish/twardyece-manager-key.pem:ro \
       -v ./test/manager-cert.pem:/etc/redfish/twardyece-manager-cert.pem:ro \
       -p 3000:3000 \
       -p 3001:3001 \
       --entrypoint=/usr/bin/$1 \
       -e 'RUST_LOG=tower_http=debug' \
       seuss-test:latest \
       --config /etc/redfish/config.yaml
