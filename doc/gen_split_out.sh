#!/usr/bin/env bash

SUBCOMMAND="split-out" envsubst < ./doc/subcommand.template

echo "\`\`\`"
./target/release/mgt split-out --help
echo "\`\`\`"
