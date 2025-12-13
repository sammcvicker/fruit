Fruit is `tree` with two improvements:

1. It defaults to ignoring files and folders not tracked in git
2. If a file includes top/module level comments, it shows the first line of those comments in the tree output after a "#" in a dim color (option to show full comment - requires more delicate formatting concerns for multi-line comments)

Other than that it is the same as tree, same colors by default, same options and flags, etc. Language choice of rust might make sense, zig as alternative - but less mature CLI libs.
