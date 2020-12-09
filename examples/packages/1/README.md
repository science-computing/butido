# Example 1

This is an example of packages.

It contains only scripts that are packaged and not real packages, but it tries
to resemble a real packaging process cleanly, including downloads and even
failing builds (configurable via ENV variables).


## Setup

The top-level `shell.nix` file contains a list of environment variables that are
required for butido and the Makefile to automagically find the right locations.

The `shell.nix` file can be called with
`nix-shell ./shell.nix --argstr example 1` to explicitely select the environment
for example 1.


## Downloads

The downloads are done from localhost, where a (python) web server has to serve
the files.
Butido can download from there.


## Packages

The packages are dependend on eachother like this:

```
a
 `- b
 |  `- d
 |  |  `- i
 |  `- e
 |  `- f
 `- c
    `- g
    `- h
       `- j
```

## Database

The database host is to be expected to run with the settings specified in the
`shell.nix` file (see `example_1_env`).

See `/scripts/dev-pg-container.sh` for how to start the container.
Use `diesel db reset` to setup the database.


## Build

The actual build is done in /tmp, where directories are created for the sources,
staging packages, released packages, the repository of package definitions and
the logs.

The `Makefile` can be used to do this.

