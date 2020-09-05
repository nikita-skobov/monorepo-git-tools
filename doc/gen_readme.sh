#!/usr/bin/env bash

# year-month-date
DATE=$(date +%Y-%m-%d) envsubst < ./doc/readme.template

echo "\`\`\`"
./target/release/mgt --help
echo "\`\`\`"
