#!/usr/bin/env bash

set -e

# linux
image=""
while [[ $# -gt 0 ]]
do
  case "$1" in
    --dev)
      dev=1
      ;;
    *)
      if [ -n "$image" ]
      then
        echo "excepted single argument for the image value"
        exit 1
      fi
      image="$1"
      ;;
  esac
  shift
done

docker --version
pwd
ls -al

source_dir="$(pwd)"
docker_dir="ci/docker"

if [ -f "$docker_dir/$image/Dockerfile" ]; then
    dockerfile="$docker_dir/$image/Dockerfile"
    # build docker image.
    docker buildx build --network host --rm -t rim-ci -f "$dockerfile" .
else
    echo "Invalid docker image: $image"
fi

# 运行 Docker 容器
echo "Running docker with EDITION=$EDITION"
docker run --workdir /checkout/obj \
  -e "EDITION=$EDITION" \
  -v "$source_dir:/checkout/obj" \
  --init --rm rim-ci
