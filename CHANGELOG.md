# Changelog

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

