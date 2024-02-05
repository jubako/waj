What is waj
===========

Waj is a website container format based on the
[jubako container format](https://github.com/jubako/jubako).

It allow you to create, and serve website archive.

Waj (and Jubako) is in active development.

If you know zim file format, waj is pretty closed from it except few (important) features:
- No book's metadata stored.
- No title index.
- No fulltext search.


How it works
============

Jubako is a versatile container format, allowing to store data, compressed or not,
in a structured way. It main advantage (apart from its versability) is
that is designed to allow quick retrieval of data fro the archive without
needing to uncompress the whole archive.

Waj use the jubako format and create waj archive which:
- Store content compressed.
- Can do random access on the waj archive to allow quick serving to a request


Try waj
=======

Install waj
-----------

```
cargo install waj
```


Create an archive
-----------------

Creating an archive is simple :

Assuming you have a directory `my_directory` containing a static website:

```
waj create --file my_archive.waj -1 --strip-prefix "my_directory/" my_directory 
```

It will create one file : `my_archive.waj`, which will contains all content in the `my_directory` directory.
As we don't want `my_directory/` being part of the url's path, we removing it from the entries pathes.


Listing the content of an archive
---------------------------------

You can list the content of the archive with:

```
waj list my_archive.waj
```

Serving the archive
-------------------

`waj` binary provides a small server.

```
waj serve my_archive.waj localhost:8080
```

It will serve the content in the archive.
Routing is pretty simple:
- It removes any trailing `/` in the request and search for it.
- If there is a query string (`?`) remove it from the path and search for the new path.
- If (original) path (without the `?`) ends with a `/`, search for  `<path> + "index.html"`. 

For example :
- `/` -> search for `` and  `index.html`
- `/foo/?value=bar` -> search for `foo/?value=bar`, `foo/`, `foo/index.html`

If your main page is not `index.html` (let's say `main`), you can create a redirection `` to `main` using
the `-m main` option at waj creation.

Zim2Waj
-------

There is a small tool at `https://github.com/jubako/zim2waj` to convert any existing zim file into a waj.
