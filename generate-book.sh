#!/usr/bin/env bash

set -e

if [ ! -d src ]; then
    mkdir src
fi

printf '[Introduction](introduction.md)\n\n' > src/SUMMARY.md

find ./text ! -type d -print0 | xargs -0 -I {} ln -frs {} -t src/

find ./text ! -type d -name '*.md' -print0 \
  | sort -z \
  | while read -r -d '' file;
do
    printf -- '- [%s](%s)\n' "$(basename "$file" ".md")" "$(basename "$file")" 
done >> src/SUMMARY.md

ln -frs README.md src/introduction.md

mdbook build
