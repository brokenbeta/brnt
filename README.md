
# bulkrn

Rename files using your text editor of choice.

    bulkrn
        [-e|--editor EDITOR-PATHNAME]
        [-x|--include-extensions]
        [--dry-run]
        SEARCH-PATTERN...

    bulkrn --set-editor EDITOR-PATHNAME

# TODO

*  ~~save editor to a config file~~
*  ~~flag to include/omit extension in the text file~~
*  ~~dry-run~~
*  handle situation where files must be renamed in certain order to avoid files clashing mid-process
*  detect invalid situations:
    *  ~~0 files matched pattern~~
    *  files which have unworkable filenames to start with e.g. newlines in filename
    *  new filenames provided contain illegal characters e.g. asterisk, question mark
    *  new filenames provided contain duplicates
    *  new filenames provided clash with other files in the target location
    *  making sure that the definition of "duplicate" and "clash" varies with filesystem case sensitivity?
*  handle file-cannot-be-modified gracefully with choice to retry / abort / rollback / skip
*  display output in way that divides into renamed | unchanged by choice | failed
*  undo
