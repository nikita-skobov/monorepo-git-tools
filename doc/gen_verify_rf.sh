#!/usr/bin/env bash

SUBCOMMAND="verify-rf" envsubst < ./doc/subcommand.template

echo "\`\`\`"
./target/release/mgt verify-rf --help
echo "\`\`\`"
