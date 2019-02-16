#!/bin/bash

set -euv

echo "$TRAVIS_EVENT_TYPE -> $TRAVIS_BRANCH"
cd "$TRAVIS_BUILD_DIR"
cargo clean
cargo doc
cd target/doc
git init
git checkout -b gh-pages
git add .
git -c user.name='travis' -c user.email='travis' commit -q -m "Automatic update for $TRAVIS_COMMIT"
git push -f -q https://dtantsur:$GITHUB_API_KEY@github.com/dtantsur/rust-openstack gh-pages 2>/dev/null
