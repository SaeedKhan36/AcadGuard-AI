#!/bin/sh
# Demo: build an index, check the essay, then re-filter the saved check.
set -e
cd "$(dirname "$0")"
PLAG="${PLAG:-cargo run -q --release -p plagengine --}"
rm -f demo.db
$PLAG --db demo.db ingest corpus/internet --collection internet
$PLAG --db demo.db ingest corpus/publications --collection publications
$PLAG --db demo.db check essay.md --store > report.json
echo "similarity: $(grep -m1 '"percent"' report.json)"
ID=$(sed -n 's/.*"check_id": "\(.*\)".*/\1/p' report.json | head -1)
$PLAG --db demo.db report "$ID" --exclude-quotes --exclude-bibliography > report-filtered.json
echo "wrote report.json and report-filtered.json"
