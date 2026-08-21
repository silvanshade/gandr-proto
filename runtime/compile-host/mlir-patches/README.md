# The pinned MLIR's patch series

Local modifications to the pinned `llvm-project` ride here as a curated [Stacked Git](https://stacked-git.github.io/) series applied over the fetched archive, never as edits committed into a vendored tree.
The mechanism follows the Mojo compiler's, and `../cmake/mlir-pin.cmake` states the pin itself.

**The series is empty, and that is the fact worth keeping visible.** The pin is upstream `llvmorg-22.1.8` unmodified.
A reader who wants to know whether this project builds against a patched LLVM can answer it from `series` alone.

Adding a patch:

```text
stg init                       # once, in the fetched tree
stg new <name>                 # describe what it changes and why, in the message
# edit, then
stg refresh
stg export --dir <this directory>
```

Every patch carries the reason it exists and the condition under which it goes away — an upstream fix landing, a workaround becoming unnecessary.
A patch with no removal condition is a fork, and this is not one.
