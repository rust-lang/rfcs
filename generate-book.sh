#!/usr/bin/env bash

set -e

add_badge() {
  TITLE=${1:-PR}
  TYPE=${2:-pull}
  SHIELD_IO_TYPE=${3:-$TYPE}

  FIND="\(https://github.com/([^/]*)/([^/]*)/$TYPE/([^)]*)\)"
  SUBS="(https:\/\/github.com\/\1\/\2\/$TYPE\/\3) ![$TITLE #\3 badge](https:\/\/shields.io\/github\/$SHIELD_IO_TYPE\/detail\/state\/\1\/\2\/\3)"

  sed -ibak -E "s|$FIND|$SUBS|g" src/*-*.md
}

if [ ! -d src ]; then
    mkdir src
fi

printf '[Introduction](introduction.md)\n\n' > src/SUMMARY.md

find ./text ! -type d -print0 | xargs -0 -I {} ln -frs {} -t src/

# add badge next to github issues & PRs
add_badge "Issues" "issues"
add_badge "PR" "pull" "pulls"

find ./text ! -type d -name '*.md' -print0 \
  | sort -z \
  | while read -r -d '' file;
do
    printf -- '- [%s](%s)\n' "$(basename "$file" ".md")" "$(basename "$file")" 
done >> src/SUMMARY.md

ln -frs README.md src/introduction.md

mdbook build
