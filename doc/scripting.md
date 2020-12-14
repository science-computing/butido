## Scripting

This document describes the scripting in butido.

First of all, the scripts butido runs are not set to be bash or any other
programming language.
Technically, writing your packaging scripts in anything that can be called with
a shebang is possible (because butido takes the shebang from the config when
compining the script).

_BUT_ the scripts for all packages must be the same scripting language, it is
not possible to write one package in bash and some other package in python and
make butido automatically chose the right interpreter for the shebang.

Besides from that, there are no hard requirements but only some that make your
life easier.

The following sections describe some output lines butido can parse and
understand to get metadata for your build.

These can be either printed via `echo` or via the script helpers provided for
each kind of output. Note that the script helper is equivalent to writing the
`echo` output yourself and is just added for convenience.

Helpers for other scripting languages besides bash do not exist (yet?).


### State

Your script can finish with two states.
These states _MUST_ be set from your script, otherwise, butido will not be able
to know whether your build was successfull or not.

* Printed:
    `echo "#BUTIDO:STATE:OK"` for a successfull exit
    `echo '#BUTIDO:STATE:ERR:"errormessage"'` for erroneous exit
* Helper
    `{{state "OK"}}` for a successfull exit
    `{{state "ERR" "message"}}` for erroneous exit


### Phases

The configuration file features a field where you can define "phases" of the
packaging script. These phases are nothing more than commented sections of your
script. Butido concatenates all configured phases to one huge script.
These phases can help you organizing what is happening, for example you can have
a phase for unpacking the sources, one for preparing the build, one for building
and finally one for packaging.
The upper limit of number of phases is not restricted, but at least one must
exist.

Phases can be announced to the CLI frontend via printing

* Bash: `echo '#BUTIDO:PHASE:<phasename>'`
* Helper: `{{phase "<phasename>"}}` using the helper provided by butido.

Only the latest phase will be shown to the user.
The phase name will also be shown to the user if the packaging script fails, so
they can find the location of the error faster.


### Progress

The script can also print progress information to the CLI frontend. This
progress information is nothing more than a number (`0..100`) that is used to
update the progress bar.

It can be updated using

* Bash: `echo '#BUTIDO:PROGRESS:<number>'`
* Helper: `{{progress <number>}}`

This feature is completely a quality-of-life feature to give the caller of
butido a visual feedback about the progress of a packaging script.
For the packaging progress itself it is not required.


(Butido might get functionality to infer the progress information based on
earlier builds of the same package using heuristics. This might or might not
deprecate this feature).

