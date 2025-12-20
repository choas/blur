# Blur

**Blur** is an esoteric programming language where every variable stores the **average** of all values it has ever been assigned.

```c
int x = 10;    // x = 10
x = 20;        // x = 15 (average of 10, 20)
x = 30;        // x = 20 (average of 10, 20, 30)
```

Welcome to regression to the mean.

## Features

- **C-like syntax** with a twist: assignments accumulate, they don't replace
- **Type-specific averaging**: int (ceiling), float (exact), bool (>=50% threshold), char (ASCII average), string (per-position)
- **Chaotic for loops**: loop counters average themselves, causing exponential iteration counts
- **`sharp for`**: escape hatch for normal loop behavior
- **Configurable blur factor**: weight recent values more with `--blur 0.5`
- **Functions**: parameter history travels with values
- **Interactive REPL** with C64-style `.load` command

## Quick Start

```bash
# Build
cargo build --release

# Run a program
./target/release/blur examples/hello.blur

# Start the REPL
./target/release/blur

# Execute code directly
./target/release/blur -e "int x = 5; x++; x = 10; print(x);"
```

## The Infamous For Loop

```c
for (int i = 0; i < 5; i++) {
    print(i);
}
```

This doesn't print `0, 1, 2, 3, 4`. The loop counter `i` averages itself:

```
i = 0  (1 iteration)
i = 1  (2 iterations - stuck)
i = 2  (7 iterations - stuck longer)
i = 3  (20 iterations)
...
```

Regular `for` loops cap at 1000 iterations with a warning. Use `sharp for` for normal behavior:

```c
sharp for (int i = 0; i < 10; i++) {
    print(i);  // Prints 0-9 normally
}
```

## Blur Factor

Control how much history affects the current value:

```bash
blur --blur 1.0 program.blur   # Pure average (maximum blur)
blur --blur 0.9 program.blur   # Slight recency bias (default)
blur --blur 0.5 program.blur   # Strong recency bias
blur --blur 0.0 program.blur   # Only most recent value (no blur)
```

Or use a directive in your program:

```c
#blur 0.5

int blur() {
    // ...
}
```

## Examples

See the `examples/` directory:

- `hello.blur` - Hello World
- `money.blur` - Watch your savings regress to the mean
- `loop_chaos.blur` - The infamous for loop in action
- `sharp_loop.blur` - The escape hatch
- `booleans.blur` - Boolean averaging (voting!)
- `strings.blur` - String blurring
- `tour.blur` - All features in one file

## Entry Point

Programs start at `blur()`, not `main()`:

```c
int blur() {
    print("Hello, Blur!");
    return 0;
}
```

## REPL Commands

```
.help          Show help
.vars          Show variables and their history
.blur [value]  Show or set blur factor
.clear         Reset interpreter state
.load <file>   Load and run a .blur file
.run [func]    Run a function (default: blur)
.exit          Quit
```

## Documentation

See [WIKI.md](WIKI.md) for the complete language specification.

## License

MIT License - see [LICENSE](LICENSE) for details.
