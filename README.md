
## Remake build system
#### (Suggestions for a better name are appreciated)

Remake is a very simple build system similar to gnu Make.
Remake does not yet support conditional recompile using
file dates, but I intend for this to be a feature at
some point

Remake has 3 main functions

targets
```
  build:
    gcc -c example.c
    g++ main.cpp -lexample -o build/main
```

dependencies
```
  run: build
    build/main
```

global variables

```
  export build_dir $(pwd)/target/debug
```

or

```
  export sdl_libs = $(pkgconf --lib sdl2)
```

the '=' is optional and the text following the name
is terminated by a newline '\n' and interpreted as follows

```
  export name = expr
```

becomes

```bash -c "echo expr"```


