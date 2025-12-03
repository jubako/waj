# Waj 0.4.1

- Add missing license file in subcrates
- Add mime-type definition for waj files.

# Waj 0.4.0

- Allow waj serve to server several waj files. Routing can be done base on path (first part) or host.
- Add an option `--nb-threads` to select the number of thread to use for the server
- Test `waj create` and `waj list` command line.
- Adapt to new Jubako api (error types, variant and property names, SmallVec, utf8 locator, array cmp)


# Waj 0.3.0

This release is based on version 0.3.2 of Jubako.
This is a major release, see Jubako changelog for changes impacting arx.
Main information to remember of Jubako release is that the format as evolved and compatibility
with previous version is broken.

- Update code with new API of Jubako library.
- `--version` option now includes the git commit.
- Better command line option (select compression, concat mode, check of input and output path)

There is no functional change but as new jubako format is not compatible, we udpate the version.


# Waj 0.2.1

- Add option to generate man page and completion script.
- `waj create` now uses option `-o` to specify the path of the created archive.
  `-f` is keep for compatibility but will we remove soon.
- Better help message.
- Update CI

# Waj 0.2.0

This is the first version of Waj.

You can create a Waj archive and serve it.
