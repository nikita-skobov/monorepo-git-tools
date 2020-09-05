#!/usr/bin/env bash

# if any command fails, we want the script to exit with error
set -e

echo "$@"

if [[ "$@" == *"windows"* ]]; then
  echo "running windows"
  ls ./target/"$1"/release/
  ls ./target/"$1"/release/mgt.exe
  cp ./target/"$1"/release/mgt.exe /usr/bin/mgt
else
  echo "running linux"
  sudo cp ./target/"$1"/release/mgt /usr/bin/mgt
  sudo chmod +x /usr/bin/mgt
fi

echo "does /usr/bin/mgt work?"
/usr/bin/mgt -h

echo "does mgt work?"
mgt -h
