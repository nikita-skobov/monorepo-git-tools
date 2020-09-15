#!/usr/bin/env bash

SUBCOMMAND="check-updates" envsubst < ./doc/subcommand.template

echo "\`\`\`"
./target/release/mgt check-updates --help
echo "\`\`\`"
