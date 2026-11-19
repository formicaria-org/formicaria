#!/bin/sh
# **`pixi run block-resources`** (`privacy-plan-2026-11-19.md` 3.2, 3.3): uBlock Origin's stand-ins
# and scriptlets for the phone's pages, assembled from one pinned uBlock release (GPL-3.0, as the
# APK) into `mobile/src-tauri/block/ubo-resources.json`, which the phone's library carries. The
# release's archive is checked against its pinned SHA-256; a new release is pinned by hand here.
set -eu
TAG=1.76.4
SHA=554815c21f0d0733de98c3a314f8cf827365184d004ae49b85697bdd25963344
work=target/ubo-resources
rm -rf "$work"; mkdir -p "$work"
curl -sSfL "https://codeload.github.com/gorhill/uBlock/tar.gz/refs/tags/$TAG" -o "$work/ubo.tar.gz"
got=$(sha256sum "$work/ubo.tar.gz" | cut -d' ' -f1)
if [ "$SHA" != "$got" ]; then echo "block-resources: uBlock $TAG is $got, pinned $SHA"; exit 1; fi
tar -xzf "$work/ubo.tar.gz" -C "$work"
src="$work/uBlock-$TAG"
node ci/ubo-scriptlets.mjs "$src" "$work/scriptlets.json"
mkdir -p mobile/src-tauri/block
cargo run -q -p fm-block --features assemble --example assemble -- "$src" "$work/scriptlets.json" mobile/src-tauri/block/ubo-resources.json
ls -l mobile/src-tauri/block/ubo-resources.json
