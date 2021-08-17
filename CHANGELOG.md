# Changelog

## v0.2.0

The v0.2.0 release of butido is considered incompatible with the v0.1.x
releases. The v0.1.x branch is now unmaintained and no further bugfixes for
butido v0.1.0 will be released.

There are a lot of changes over the complete source tree. A lot of things
changed and were improved. For details, have a look at the individual commits.

The v0.2.0 release contains all fixes from the v0.1.x bugfix releases.


### CLI

Changes in the Commandline interface of butido (new features, changed
interfaces, removed things...).

* Subcommand to get path to source file
    ("butido source of gcc 9.2.0")
    * 853680633f56c29832198c85937ab39d9eafee72

* Subcommand to get a whole submit from the database
    ("butido db submit UUID")
    * f6229edd41292e19d8c18a81a86542741260e938

* Subcommand for getting only the log of a job
    ("butido db log-of UUID")
    * fc9635a3d392925f95175ab8a9c89a02debe269c

* Subcommand for getting top output of containers
    ("butido endpoint container CONTAINER top")
    * aa3b529f3f23275ff14a222401af80d63e293697

* Subcommand for stopping container
    ("butido endpoint container CONTAINER stop")
    * 60f0712a95d7cdf46554a87893fb71354f2631c5

* Subcommand for setting up the database
    ("butido db setup")
    * 41d058415ceb6cf725416d5cc9cfa49092da0cea

* Flags for filtering releases from database
    * By Date
      ("butido db releases --older-than DATE --newer-than DATE")
      * 1e77f45fc8649b3fbb9e17bd6a5e262617abcc33

    * By Package Name
      ("butido db releases --package PKG")
      * af39b570d41b634d12f9e19a0e799dbb18557312

    * By release store
      ("butido db releases --to STORE")
      * c0609a050da45bdb93fbb5f2ad28055e1343c08f

* Flags for filtering jobs from database
    * By Date
      ("butido db jobs --older-than DATE --newer-than DATE")
      * 8f9c7d77acfa16ebe9660eba113fbebd68b96eed

    * By Endpoint
      ("butido db jobs --endpoint ENDPOINT")
      * 037275412fb72a27c8bb0347d047e1a2b65e0569

    * By Package
      ("butido db jobs --package PKG")
      * 51b70e647230fb6df95dd3b0d35ce77409e73afb

    * Limiting number of jobs
      ("butido db jobs --limit 15")
      * fa5aecbc7df99982336916172bebd93cda294de6

* Flag for limiting the number of submits queried
    ("butido db submits --limit 15")
    * 0f231979ce6d35e3e9ec4773904cf763a1d9aac4

* The "source download" subcommand was improved
    * It fails late now (17a469d3dc1324bcbeba743fe3df3fede0a385f7)
    * It reports the downloaded URL in the progress bar (7e6b65ea3d72231c5f7181b70087b9b97ca69222)
    * It reports errors in the progress bar (53870e95dc563a50a13861aedcf98fbf4a176abc)

* Date(-Time) can now be specified in human readable form in the CLI
    * 67137f5b040cdf72e1d190c14eee336c86bf26c2
    * 8cd7b44501d2843cfb2d9307ae2b83db15092028
    * 513d7a17a580274c18aaa8161557b616574f07f8

* The endpoint and container is now listed in the progress output
    * 2c0c0eb515aa4d3d3de1ab47351733ba59c983f1

* Progress bars are updated each second
    * 25cdd0f1a0ae0afab5e34b18ea00f1727d7176a9

* Submit details are printed before start building
    * a0bcd7e74e426cae36b7372230b5ba369ee587de

* Database IDs are not shown in output anymore
    * 0b110db5b65297bc82ae40cd24dc07c7dce190a4


### Features

Changes of functionality, new features and removed stuff that is not directly
visible on the commandline interface.

* The "Flags" feature was removed
    * 07e16931ea1bc6822464b21420147bd899a09510

* human panic was set
    * 98d5898522da748d48ab97bb9d6893c05aaf7e5f

* Timeout when connecting to endpoints
    * 441ae36017058ecbf66951c26176f24fb809471d

* Timeout when connecting to database
    * 1ebfa387fa5d47e3200b990f63f55bbe36adeff1

* Filtering of /outputs directory name
    * 3537f8e1d174f2ce422c9024a9e0d6df87ad9207

* Shuffling of endpoints
    * 8706494827ad598d335a9369beecdd0ddfef0cee

* Error if a patch file is missing
    * 5c36c119f9448baf6bfe5245c6ebac1aa09d5b43
    * f78df9eeb3c8d6aa6f74c0bb774282662b1ef635


### Other

Other changes included bugfixes, improvement of error messages and other output
as well as speed improvements, refactoring of large parts of the code for better
readability and maintainability as well as dependency updates.


## v0.1.4

Bugfix release. Fixes:

* A typo was fixed
    * aef74e56afe695f39e6ce38588a4ddbed9cf1817

* A bug was fixed where not-found dependencies were ignored
    * e8063c060cddef9a6f1d8757eaf17c23d6c387a3
    * 9b47e557a40dab7bd13e92790c4515d05f073157

* A bug was fixed where colorized strings were put into the database,
  unintentionally, causing log parsing to fail silently
    * 8e0ba26b15ffb78fa05f1e631bfd12722ddcd31e

* sourcehut builds were removed
    * e733ead7f7849030bc87e1b9da4d8b9046b71904


## v0.1.3

Bugfix release. Fixes:

* If two submits were equal except for environment variables, butido reused the
  artifacts of the first submit because it considered them equal. This was fixed
  by rewriting the equality-check logic
      * f5ba5d8f510fcb73c87dea9962850ca3bbfb7fc0

* Early-fail in the "source download" implementation was fixed to be late-fail
    * 875cbce45c9ca7c646d768884101b0bdd5309606

* Helptext of the "source download" subcommand was fixed
    * 9b0cf8e75fb7f83f8fe6d4ea07ceb4bd0932d865

* Do not print database password in debug log
    * 3ca72a47678f9d57c40630d99382e0014317d95c


## v0.1.2

Bugfix release. Fixes:

* Argument processing bug was fixed, where a CLI parameter was unintentionally
  ignored
    * 78c65311313876728ccead6aaa76fdc23e2b22ed

* Release artifacts are copied instead of renamed
    * 7abfe3e095e41a0742f7a7f30ce8d058edfcd94f

* Typo in trace output
    * 8cf287dd93f95775787b3a83500fee23cd8f07db

* If the build fails, parent jobs should not fail anymore because of missing
  dependencies, but simply propagate the error
    * 7f3eeeb39c34903f2bd709a78287bc2b655b062c


## v0.1.1

Bugfix release. Fixes:

* Interactive question on "endpoint containers prune" command did not honor
  "no" reply
    * 5cab13445afc2ac1852f0a3380ceab14d24302b3

* Dependencies with CVEs updated: diesel, tar
    * 555dc844dcd7c50e198b072abb8aa3f43d969660
    * 816a1a0b8a5aee4721b5a408b4017f8cec5d485a

* Value name for "source download" package version argument was fixed
    * 9c9263bff2fc5bab292126d7e83a6b06e5180ef9

* Error out if a patch file is missing
    * b2a68f896b444df551549caead9ab44bd76fdf22
    * 10b3629732eaccf0b044037714f0ffa452faaf4a


## v0.1.0

* MVP release

