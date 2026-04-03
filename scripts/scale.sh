# scripts/scale.sh
#!/bin/bash

COUNT=$1

docker-compose up -d --scale worker=$COUNT

echo "Scaled workers to $COUNT"