#!/usr/bin/env bash

SUBCOMMAND="check" envsubst < ./doc/subcommand.template

echo "\`\`\`"
./target/release/mgt check --help
echo "\`\`\`"
