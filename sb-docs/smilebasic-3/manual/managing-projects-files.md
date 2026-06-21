---
title: Managing Projects / Files
slug: docs-sb3-manual-managing-projects-files
system: SmileBASIC 3
type: guide
topic: 14
source: e-manual.pdf
scraped: 2026-06-21
---

# Managing Projects / Files

This menu allows you to create new "project folders" in which programs and data are saved, and to rename, copy, and delete files.

## Rename

This is used to rename project folders and files.

1. Select the project folder or file you wish to rename.

2. Enter a new name using the simplified keyboard.

3. File/folder name will be changed.

You can also change file names by using the `RENAME` instruction.

## Delete

This is used to delete files that are no longer needed.

1. Select the Project folder containing the files you wish to delete.

2. Select the files you want to delete.

3. A message confirming deletion of the files will appear.

- Yes — File(s) will be deleted.
- No — File(s) will not be deleted.

You cannot restore files once they have been deleted. Please be careful not to delete the wrong file by mistake.

You can also delete files by using the `DELETE` instruction.

## Copy

This is used to copy files with new names.

Copying a project:

1. Select the project to copy
2. Input a new project name
3. Copy the project with the new name

Copying one or more files:

1. Select the project to copy from
2. Press the Select File button
3. Select the file(s) you wish to copy (You can select multiple files)
4. Select the project to copy to
5. Copy the selected file(s)

A confirmation screen will appear if any file names already exist.

## Add Project Folder

This is used to create new project folders.

1. Enter the name of the new project folder using the simplified keyboard that appears.

2. The project folder will be created.

Select whether or not you want this project to be the default project (whether or not you want to use it immediately).

- Yes — The new project folder will be set as default.
- No — Will not be set as default for the moment.

If you change the default project, the programs and files in the previous project folder will no longer be visible. You can change the default project by selecting "Change Active Project" below, or by using the `PROJECT` instruction.

## Change Active Project

This is used to change the default project folder where files handled in BASIC are saved.

In the initial state, a project called DEFAULT is assigned.

You will no longer be able to load the files in the previous project folder in BASIC, but they do still exist.

If you reassign the previous project folder as the default project, your old files will be available again.

You can also change the default project by using the `PROJECT` instruction in BASIC. Entering `PROJECT ""` will restore the default project to its initial state.

## Concept of the Active Project

SmileBASIC assumes the following three conditions for the current project:

1. Current project at start-up time (specified using "Change Active Project" under "Manage Projects/Files")

2. Current project at non-execution time (set with the `PROJECT` instruction)

3. Current project at execution time (set during execution, e.g., of `EXEC`)

When one of the above is changed, the subordinate project settings will also be updated accordingly.

For example, if the current project at start-up time is changed, the current project at non-execution time and execution time will also be changed.

When execution is started (by using `RUN`, executing a tool, or executing a program from the file viewer), the current project at execution time will be set as the initial value of the current project at non-execution time.
