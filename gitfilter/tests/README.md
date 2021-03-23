the `run_tests.sh` script will run all of the bats tests.

For convenience, it will try to read from a file called "test_vars.sh" in the same
directory as the `run_tests.sh` script.

You can create this file and enter in the following environment variables if you wish to repeatedly run these tests:

```
export GITFILTERCLI="/full/path/to/gitfiltercli"
export PATHTOREACTROOT="/full/path/to/react/repo/root"
```

Otherwise, you can provide those every time manually to the
`run_tests.sh` script:


```sh
./run_tests.sh /full/path/to/gitfiltercli /full/path/to/react/repo/root

# OR:

GITFILTERCLI="/full/path/to/gitfiltercli" PATHTOREACTROOT="/full/path/to/react/repo/root" ./run_tests.sh
```
