#!/usr/bin/env bash

function bench() {
  echo "========================================================="
  cargo run --bin mini-redis-server --release ${@:1} 2>/dev/null &
  sleep 1
  echo "BENCHMARKING" ${@:1}
  redis-benchmark -t set -n 1000000 | tail -n 5
  killall mini-redis-server
  sleep 1
}

bench --no-default-features
bench --no-default-features --features tracing
bench --no-default-features --features xray
