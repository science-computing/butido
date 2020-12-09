# Example 1

This is an example of packages.

It contains only scripts that are packaged and not real packages, but it tries
to resemble a real packaging process cleanly, including downloads and even
failing builds (configurable via ENV variables).


## Downloads

The downloads are done from localhost, where a (python) web server has to serve
the files.
Butido can download from there.


## Packages

The packages are dependend on eachother like this:

```
a
 `- b
 |  `- c
 |  |  `- h
 |  `- d
 |  `- e
 `- c
    `- f
    `- g
       `- i
```

## Build

The actual build is done in /tmp, where directories are created for the sources,
staging packages, released packages, the repository of package definitions and
the logs.

The `Makefile` can be used to do this.

