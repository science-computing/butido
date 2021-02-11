# Example package tree

This subtree contains a large number of "packages", whereas each "package" is
nothing more than a number piped to a file.
This can be used to test out butido and how it works.


## The packages

Each package has a single-letter name ('a'..'z') and a version number that
increases for each package, so 'a' has version '1', and 'b' has version '2' and
so on.

The hierarchy of packages is described as follows:

                              A
                              +
                              |
              +---------------+----------------+
              |                                |
              v                                v
              B                                C
              +                                +
              |                      +--------------------+
       +-------------+               |         |          |
       v      v      v               v         v          v
       D      E      F+--------+     G         H          I
       +      +      +         |     +                    +
       |      |      |         |     |                +---+---+
       v      v      v         +---+-+-+---+          v       v
       J      K      L         |   |   |   |          Q       R
       +      +      +         v   v   v   v          +
    +--+---+  |      |         M   N   O   P          |
    v      v  |      v                 +   +          v
    S      T  +----->U                 |   |          V
                     +                 |   |          +
                     |                 |   |          |
                     |                 |   |          v
                     |                 |   |          W
                     |                 |   |          +
                     |                 |   |          |
                     |                 |   |          v
                     +-----------------+---+------->  X
                                                      +
                                                      |
                                                      v
                                                      Y
                                                      +
                                                      |
                                                      v
                                                      Z


The important features here:

* A simple chain of dependencies: Q -> V -> W -> X -> Y -> Z
* DAG-Features (non-treeish dependencies):
    * F -> M
    * K -> U
    * O -> X
    * P -> X


The order of builds should be opposite of the arrows, with builds for Z, R, H,
N, M, T and S starting right away.


Multiple versions of one package are not yet considered in this setup.


# The packaging

The packaging script does the following:

* In the "sourcecheck" phase, it checks whether the input is present (that would
  be the .tar.gz file of the package to compile)
* In the "depcheck" phase, it checks whether all dependencies are present. This
  is done to demonstrate what could be done, not because it makes a great deal
  of sense
* In the "build" phase, it "builds" a package, by piping the version number of
  the package itself to the output file.

