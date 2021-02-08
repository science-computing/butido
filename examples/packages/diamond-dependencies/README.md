# Test diamond dependencies

This subtree provides a testcase for the following:

If we have a package tree that looks like this:

      .-> C -.
     /        \
    D          > A
     \        /
      `-> B -´

(arrow means "depends on").


## Note

To reproduce the issue, make sure to adapt the ./repo/config.toml as appropriate
for your environment.

