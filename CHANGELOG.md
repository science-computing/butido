# Changelog

## v0.4.0

Details are in the individual commit messages. An overview of the changes since
the last release can be obtained via `git shortlog v0.3.0..v0.4.0`.

## v0.3.0

The v0.3.0 release of butido is considered incompatible with the v0.2.x
releases. The v0.2.x branch is now unmaintained and no further bugfixes for
butido v0.2.0 will be released.

There are a lot of changes over the complete source tree. A lot of things
changed and were improved. For details, have a look at the individual commits.

The v0.3.0 release contains all fixes from the v0.2.x bugfix releases.


### Crate maintenance and Community

* From now on (starting with the 0.3.0 release), the crate is in
  "passively-maintained" mode.
    * 9c3cb04e93dff404143b9c731b62d3a2a2e1092d

* The github repository got templates for issues and PRs as well as a COC. The
  notes on submitting patches via mail were removed
    * 56a22ff903df0d881bce88bf68cdb4de8bab06c1

* Github actions were enabled
    * 67918eea3594a2c100465820e59d2341c18aa2f3


### Major/Breaking changes

* The filename of the downloaded source does no longer contain the hash of the
  file
    * a7d5de071e0d7bc0adc8303caf592c344cb6939a

* Optional dependencies are now supported
    * 555734ea066d11d0b3efb96bff84563847f0757d


### General improvements

* The scheduling was altered to schedule new jobs to endpoints with fewer
  running containers
    * 63566cccba0c2c42af2e38d7ec0a922a60cf128d

* The "find-artifact" subcommand implementation was rewritten
    * 7fa55210e54e49e419d9e0bcff3ca7f144aac597

* The repository loading algorithm was rewritten for speed
    * ce01861e6ef3b652eeafbef5b87c293b3a971d89


### New CLI features

* The "db submits" command now lists the commit a submit was started from
    * 812fe038c3fccf9d3b14d399dc6287828b60837b

* The "db submits" command now outputs information about package and version
    * 9f021c020ada7403bec2a0dbff0f1b052b916b70

* An option was added to get all submits submitted for a specific image
    * 800ec41fadf58762ec30d864622806087cadc09c

* An option was added to get the submits for a specific commit
    * 9138cbd334115c2e649355bfef497b08c055948b

* An option was added to verify multiple packages at once
    * 7b32e20b96758f9e65cf78443520890214fdc2cb

* An option was added to download multiple packages at once
    * 7b32e20b96758f9e65cf78443520890214fdc2cb

* A subcommand for getting the images on each endpoint was added
    * 69d2334a5121edbbaa7356428e2d6e8fbd5d3443


### Fixes and other noteworthy changes

* The log output path was fixed
    * 3922865ad3950564e96ca2e4be974564fd798aef

* Fix a punctuation parsing bug in the dependency string parsing algorithm
    * 76e6562ead14963c77117f750305db5a74cf3958

* If a download URL redirects, it is followed (max 10 hops) now
    * fdd75f7ae660441db47733bac200cea4a99ba4fe

* The printing of packages was got a speedup
    * 76375ea75f7e782eaa326f63cae079847e1d5efb

* The loading-bar for DAG building is now removed from the tree-of subcommand
    * 45cda0f70ab0f1d717c3dc2b481ad84a4ff0f00f

* Rewrite of the default package print format
    * a1e759348fe9184ec6f234a91d1d6d80468cdcfa

* Fix a bug where butido did not rebuild a package if one of its dependencies
  was rebuild
    * 507aeeb8899a714fe794814615b61eeef91c7ed2

* The "release" subcommand should report errors late now, not as soon as
  they occur
    * 5b99282ba23e1002a576934a371c8b9fe7755963

* Fix CLI about text for "source download" subcommand
    * 764d693d542bf2dd2c1ae9667c2e2fedfdc13e46

* Remove a release artifact before overwriting it
    * 72c20619999beb623f762a294c252d81c8f54081

* Fix: do not require --commit flag in submits subcommand
    * e8249b10d2b41d9400d7e63fa69876d1afffc34d

* Clearify progress-bar error message
    * 9b0c6cb3253798d3eb6aa12ffddb66eda0bcb847

* Add builder options in error message
    * 544b4348c4073e63635e08a1758f947d0c8099f6

* Fix: Remove indention from after_help() text
    * d51fad98de71f90ac62883346d0f707fef89ab89

* Make sure error is printed outside of progress bars
    * 97356294acbe01a8b17486dff42ecc76726edf0a

Misc changes and fixes are not added here, as there were too many for listing
them all in the changelog.


## v0.2.1

Bugfix release. Fixes:

* The logfile path is now constructed correctly
    * c9ab871ca121efd783cb0c3d2afc001f64fdfda7

* punctuation (.-) in package name is now allowed
    * 3066146b852e6b98454cbd1b7b035a7fccb80d97

* Indention from help text was removed
    * 21e4b7d2f1c9ea603870b6c49d27d7177911bc60

* HTTP redirects are followed (only 10 for now)
    * 80792234d085a6ef990d1746f7a90b7ba8e408b7

* If a dependency gets rebuild, the dependent package gets rebuild, too
    * 216a265f6b87eb18da08fcb78aa8bd36ab1abd02

* Package print format default was updated
    * 2b42b5915f9ad974c8af76c524768e993925d632

* trace output was fixed
    * 027c1f23f5221c7463b12086e1a77887d282e398


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
