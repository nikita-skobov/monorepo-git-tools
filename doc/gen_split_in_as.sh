#!/usr/bin/env bash

SUBCOMMAND="split-in-as" envsubst < ./doc/subcommand.template

echo "\`\`\`"
./target/release/mgt split-in-as --help
echo "\`\`\`"
