#!/usr/bin/env bash
set -x
docker network create simple_redis
set -eo pipefail
export _PWD="$(pwd)"
export ROOT="$(git rev-parse --show-toplevel)"
source "${ROOT}/scripts/setup.sh"
cd "${ROOT}/" || exit
if ! command -v psql &> /dev/null
then
    >&2 echo "Error: `psql` is not installed."
    cd "${_PWD}" || exit
    exit 1
fi

if ! command -v docker &> /dev/null
then
    >&2 echo "Error: `docker` is not installed."
    cd "${_PWD}" || exit
    exit 1
fi

if [ "${SKIP_DOCKER}" != "yes" ]
then
  DOCKERS="$(docker ps -a -q --filter ancestor=redis:alpine3.20 --format="{{.ID}}")"
  if [ -n "$DOCKERS" ]
  then
    ensure docker rm --force --volumes $DOCKERS
  fi
  ensure docker run \
    --network=simple_redis \
    --name redis -p 6379:6379 -d redis:alpine3.20
  export REDIS_URL="redis://127.0.0.1:6379"
fi

echo "Redis ready"
cd "${_PWD}"
