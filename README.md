# BUTIDO

"butido" could stand for "but i do", "BUild Things In DOcker" or
"Better Universal Task Instrumentation Docker Obersavtor".

Anyways, it is a tool for building packages for linux distributions in docker
and it does not make assumptions about the build procedure itself (and thus can
build .rpm, .deb, or any other package format your scripts can build).


## Functionality

Packages are defined in TOML and in hierarchies
(see [config-rs](https://docs.rs/config/)).
See the [examples](./examples) for how to define packages.

The "business-logic" of packages are shell scripts which exist in predefined
"phases".
These scripts are compiled into one large script (per package) which is then
run to build the source into a package.

The package definition(s) can hold meta-information and (of course) information
about a packages dependencies. Both dependencies and meta-information is made
available in a build.

Everything that is computed before, during or after a build or submit is written
to a postgres database, including build logs.
This database can be queried for packages, build information, logs and other
data.

Successfully built packages are collected in a "staging" store on FS. A staging
store is created per submit.
The results can be taken from this "staging" store and be released into a
"release" store.


## Requirements

Building butido is easy, assuming you have a Rust installation:

```bash
cargo build --release # (remove --release for a debug build)
```

Running is not so easy:

* Create a git repository
* Put a `config.toml` file into the repository root, adapt parameters to your
  needs
* Write `pkg.toml` files for your packages
* Run a postgres instance somewhere
    * configure it to be accessible for butido with user and password
    * Setup tables (butido cannot do this yet)
* Run docker instances somewhere, accessible via HTTP or socket

Now you should be ready to try butido. butido takes `--help` in every subcommand
to explain what it does.


### Glossary

| Word        | Explanation                                                                                                      |
+ ----------- + ---------------------------------------------------------------------------------------------------------------- +
| build / job | The procedure of transforming a set of sources to a package (or, technically, even to multiple packages)         |
| dependency  | A "package" that is required during the buildtime or during the runtime of another "package"                     |
| endpoint    | A docker API endpoint butido can talk to                                                                         |
| jobset      | A list of jobs that can be run in any order or in parallel                                                       |
| output      | The results of a butido build job                                                                                |
| package     | A single (archive) file OR the definition of a job                                                               |
| script      | The script that is run inside a container. Basically the "->" in "source -> package".                            |
| source      | A file that contains a source code archive                                                                       |
| submit      | A call to butido for building a single package, which can result in multiple packages (dependencies) being built |
| tree        | The tree structure that is computed before a packages is built to find out all (transitive) dependencies         |


# License

butido was developed for science+computing ag (an Atos company).

License: EPL-2.0

