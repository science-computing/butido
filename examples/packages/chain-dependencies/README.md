# Test chained dependencies

This subtree provides a testcase for the following:

If we have a package tree that looks like this:

    C -> B -> A

("->" means "depends on").

We construct three jobs, where B waits for results from A and C waits for
results from B and A.

Though, the implementation has a bug right now, where C waits for results from B
and A, but the results from A are not forwarded from B to C.

This subtree provides a test for reproducing the issue as a baseline to solve
it.


## Note

To reproduce the issue, make sure to adapt the ./repo/config.toml as appropriate
for your environment.

