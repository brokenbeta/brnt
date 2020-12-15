
# brnt

The *b*est *r*e*n*aming *t*ool.

Rename files in bulk using your text editor of choice.

**Experimental**. Use at your own risk, and sorry if it screws up.

![example using Sublime Text](example-sublime.gif)

## Usage

    brnt
        [-e|--editor EDITOR-PATHNAME]
        [-x|--include-extensions]
        [--dry-run]
        SEARCH-PATTERN...

    brnt --set-editor EDITOR-PATHNAME

## My TODO list

*  ~~save editor to a config file~~
*  ~~flag to include/omit extension in the text file~~
*  ~~dry-run~~
*  handle situation where files must be renamed in certain order or with temporary filenames to avoid files clashing mid-process
*  detect invalid situations:
    *  ~~0 files matched pattern~~
    *  files which have unworkable filenames to start with e.g. newlines in filename
    *  new filenames provided contain illegal characters e.g. asterisk, question mark
    *  new filenames provided contain duplicates
    *  new filenames provided clash with other files in the target location
    *  making sure that the definition of "duplicate" and "clash" varies with filesystem case sensitivity?
*  the ability to invoke text editor with custom arguments (beyond just the buffer filename)
*  ~~handle file-cannot-be-modified gracefully with choice to retry / abort / rollback / skip~~
*  ~~display output in way that divides into renamed | unchanged by choice | failed~~
*  undo
