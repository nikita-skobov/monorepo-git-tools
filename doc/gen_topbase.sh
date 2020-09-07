#!/usr/bin/env bash

SUBCOMMAND="topbase" envsubst < ./doc/subcommand.template

echo "\`\`\`"
./target/release/mgt topbase --help
echo "\`\`\`"
