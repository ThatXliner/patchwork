# patchwork examples

A toy codebase for trying out AST-based refactoring. Run these from the
repo root.

## Java: migrate debug() to logger

```bash
cargo run -- replace -i -p 'debug($msg)' -r 'logger.warn($msg)' examples/java/UserService.java
cargo run -- find -p 'user.$method($($thing),*)' examples/java/UserService.java
```

## Python: remove print debugging

```bash
cargo run -- replace -i -p 'print($msg)' -r 'logger.info($msg)' examples/python/api.py
cargo run -- find -p 'logger.info($msg)' examples/python/api.py
```

To delete prints entirely:

```bash
cargo run -- delete -i -p 'print($msg)' examples/python/api.py
```

## JavaScript: destructure reorder

```bash
cargo run -- replace -i -p 'const {$a, $b} = $obj' -r 'const {$b, $a} = $obj' examples/javascript/app.js
```

## Find structural patterns

```bash
cargo run -- find -p 'console.log($a, $b)' examples/javascript/app.js
cargo run -- find -p 'return $val;' examples/java/UserService.java
cargo run -- find -p 'print($msg)' examples/python/api.py
```

## Undo with git

All the `-i` commands modify files in-place. If something goes wrong:

```bash
git checkout examples/
```
