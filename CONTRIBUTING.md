- Contributions are welcome and appreciated
- if you want to fix a bug/improve the code, feel free to just make your changes and submit a PR on GitHub.
- if you want to add a feature, please make an issue with the "enhancement" label, and I will give feedback on if/how/where to implement it.
- To get started with developing simply:
  ```
  git clone https://github.com/nikita-skobov/monorepo-git-tools
  cd monorepo-git-tools
  ./test/run_tests.sh
  ```
  That will probably fail at first because this project has some dependencies. mainly [`bats` (note, I use `bats-core`)](https://github.com/bats-core/bats-core), so make sure it is properly installed. Then try running `./test/run_tests.sh` again.
  If it still doesn't work, something is wrong so please file an issue.
