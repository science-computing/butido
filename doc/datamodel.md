## The data model used for the database

The following visualization _only_ shows the entities we need to store.
The tables extracted from this visualization are listed below.

```no_run
+------+ 1             N +---+ 1          1 +-----------------------+
|Submit|<--------------->|Job|-------+----->|Endpoint *             |
+--+---+                 +---+       |    1 +-----------------------+
   |                                 +----->|Package *              |
   |                                 |    1 +-----------------------+
   |                                 +----->|Log                    |
   |  1  +-----------------------+   |    1 +-----------------------+
   +---->|Config Repository HEAD |   +----->|OutputPath             |
   |  1  +-----------------------+   |  N:M +-----------------------+
   +---->|Requested Package      |   +----->|Input Files            |
   |  1  +-----------------------+   |  N:M +-----------------------+
   +---->|Requested Image Name   |   +----->|ENV                    |
   | M:N +-----------------------+   |    1 +-----------------------+
   +---->|Additional ENV         |   +----->|Script *               |
   |  1  +-----------------------+          +-----------------------+
   +---->|Timestamp              |
   |  1  +-----------------------+
   +---->|Unique Build Request ID|
   |  1  +-----------------------+
   +---->|Package Tree (JSON)    |
   |  1  +-----------------------+
   +---->|Build Plan (JSON)      |
         +-----------------------+
```


NOTIZEN:
    * "Input files" müssen nachvollziehbarsein, da diese sich ändern könnten
    * ENV im Job ist im Package "encodiert"

* Because we track the commit hash a "Submit" was submitted with,
  we can reduce the amount of data we store for "Endpoint", "Package" and
  "Script", because we can find all this information in the repository already.


## Inferred Tables

These are the tables extracted from the above visualization,
with some comments on what they are supposed to store.

* "envvar"
    A Key-Value list of environment variables.
    columns:
        - name: String
        - value: String

* "git hash"
    Simply a recording of relevant git hashes
    columns:
        - hash: String

* "package"
    A package from the package repository.
    All information we need is the name and the version.
    Because we store the git hash of the request, we can reconstruct everything
    else.

    columns:
        - name: String
        - version: String

* "image"
    A list of (docker) image names

    columns:
        - name: String

* "submit"
    The submitted request.
    This gets a unique identifier for easy referencing and a timestamp for easy
    filtering things.
    The repository hash for the submit is stored as well as the requested image
    to build on and the additional environment.

    The generated package tree (JSON) is also stored

    columns:
        - uuid: UUID
        - timestamp: Timestamp
        - envvar: (foreign "submitenvs")
        - requested_image: (foreign "image")
        - requested_package: (foreign "package")
        - repo_hash: (forgein "git hash")
        - tree: JSON
        - buildplan: JSON


* "endpoint"
    The names of the used endpoints.
    Because we track the git commit hash, we can find out the endpoint configuration from
    the repository.

    columns:
        - name: String


* "job":
    A single job which is run somewhere.

    columns:
        - submit: (foreign "submit")
        - endpoint: (foreign "endpoint")
        - package: (foreign "package")
        - image name: (foreign "image")
        - container hash: String
        - script: Text
        - output: (foreign "artifact")
        - log: Text

* "job_input_artifacts":
    N:M resolution table between jobs and input artifacts (artifacts that are
    copied into the container)

    columns:
        - job (foreign "job")
        - artifact (foreign "artifact")

* "artifact":
    A Path to a artifact generated from a build job or required as an input for
    a build job.

    columns:
        - path: Path

* "jobenvs":
    N:M resolution table between jobs and environment variables.

    columns:
        - job (foreign "job")
        - env (foreign "env")

* "submitenvs":
    N:M resolution table between submits and environment variables.

    columns:
        - submit (foreign "submit")
        - env (foreign "env")

