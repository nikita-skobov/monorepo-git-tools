#!/usr/bin/env bash

SUBCOMMAND="split-in" envsubst < ./doc/subcommand.template

echo "\`\`\`"
./target/release/mgt split-in --help
echo "\`\`\`"
