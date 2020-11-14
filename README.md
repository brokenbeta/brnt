
# brn

Rename files using your text editor of choice.

    brn [--editor EDITOR-PATHNAME] [-x|--include-extension] [file match patterns]
    brn --set-editor EDITOR-PATHNAME

# TODO

* ~~save editor to a config file~~
* flag to include/omit extension in the text file
* handle situation where files must be renamed in certain order to avoid files clashing mid-process
* detect invalid situations:
    * 0 files in pattern
    * names are invalid filenames
    * file already exists
    * filenames provided contain duplicates
* handle file-cannot-be-modified gracefully with choice to retry / abort / rollback / skip
* undo
