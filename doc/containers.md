## Containers

The containers you use to run your builds are handled the following way:

1. Dependencies and sources are copied to the container at `/inputs`,
   the compiled packaging script is copied to the container at `/script`
2. The script is started
3. The result artifacts are copied from `/outputs` to the staging store


### Conventions

There are some conventions regarding packages, dependencies, sources and so
on. Those are listed here.

1. Dependencies are named `/inputs/<packagename>-<packageversion>.pkg` inside the container
2. Sources are named `/inputs/src-<hashsum>.source`
3. Outputs are expected to be named `/outputs/<packagename>-<packageversion>.pkg`

The reason for the names lies in the artifact parsing mechanism.
If the package is named differently, the artifact parsing mechanism is not able
to recognize the package and might fault, which causes butido to stop running.

