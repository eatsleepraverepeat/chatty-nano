#!/usr/bin/env bash
set -e

sudo docker run \
  --runtime nvidia \
  -it \
  --rm  \
  -v $(pwd)/data:/workspace/data \
  --network host \
  --device /dev/snd \
  --group-add audio \
  chatty_nano:dev \
  $@
