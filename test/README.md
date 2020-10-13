tests are implemented using [BATS](https://github.com/bats-core/bats-core)

to run tests, make sure you are in the root of the repository
and run:

```sh
./test/run_tests.sh
```

which will compile the current library
and then run tests on the compiled library

## UPDATE Oct 2020:

I wrote a yaml task runner that can run things in parallel.
I wrote it because running these tests in series took up to 30 seconds,
and also because the above `run_tests.sh` script is kinda ugly.

So also in this directory is `./test/test.yml` which does the same as `run_tests.sh` but faster

If you want to try out my task runner that I use to run tests, checkout [simple-yaml-task-runner](https://github.com/nikita-skobov/simple-yaml-task-runner)

The task runner can run the tests via:

```sh
sytr ./test/test.yml
```
