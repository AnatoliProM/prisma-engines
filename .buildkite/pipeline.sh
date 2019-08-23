#!/usr/bin/env bash

CMD=$1

if [ -n "$CMD" ]; then
  $(which ruby) $(dirname $0)/engine-build-cli/cli.rb "$@"
else
  $(which ruby) $(dirname $0)/engine-build-cli/cli.rb pipeline
fi