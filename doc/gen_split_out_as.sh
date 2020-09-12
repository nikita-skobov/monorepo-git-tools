#!/usr/bin/env bash

SUBCOMMAND="split-out-as" envsubst < ./doc/subcommand.template

echo "\`\`\`"
./target/release/mgt split-out-as --help
echo "\`\`\`"
