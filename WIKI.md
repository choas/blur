# Blur

**Blur** is an esoteric programming language where every variable stores the **average** of all values it has ever been assigned. Created in 2025, Blur is a C-like language that demonstrates how a simple semantic change can create chaotic and unpredictable behavior.

## Overview

In most programming languages, assigning a value to a variable replaces the old value. In Blur, the variable's value becomes the **mean** of all values ever assigned to it. This creates a "regression to the mean" effect where variables resist change and loop counters behave chaotically.

| Paradigm | Imperative |
|----------|------------|
| Designed by | choas |
| Appeared in | 2025 |
| Influenced by | C |
| File extension | .blur |
| Entry point | `blur()` |

## Semantics

### Variable Averaging

Every variable maintains a history of all assigned values. Reading a variable returns the average:

```c
int x = 10;    // history: [10], value = 10
x = 20;        // history: [10, 20], value = 15
x = 30;        // history: [10, 20, 30], value = 20
```

### Type-Specific Rounding

- **int**: Uses ceiling (rounds up). `avg([5, 6]) = 5.5 → 6`
- **float**: Exact average, no rounding
- **bool**: `true` if ratio of trues >= 0.5, else `false`
- **char**: Average of ASCII values, ceiling
- **string**: Per-position character averaging (space = no-op)

### Blur Factor (Recency Weighting)

The blur factor controls how much history affects the current value. The default is 0.9 (slight recency bias).

```bash
blur --blur 1.0 program.blur   # Pure average
blur --blur 0.5 program.blur   # Strong recency bias
blur program.blur              # Default: 0.9
```

Or use a directive in your program:

```c
#blur 0.5

int blur() {
    bool b = true;
    b = false;
    print(b);  // false (recent value wins)
    return 0;
}
```

**How it works:** Each value's weight = blur^age, where age 0 is the most recent.

| blur | Behavior |
|------|----------|
| 1.0 | Maximum blur - pure average |
| 0.9 | Slight recency bias (default) |
| 0.5 | Strong recency bias |
| 0.0 | No blur - only most recent value counts |

**Example:** `true, false` with different blur values:
- blur=1.0: 50% true → `true`
- blur=0.9: 47% true → `false` (slight recency bias)
- blur=0.5: 33% true → `false` (recent `false` wins)

### Increment/Decrement Operators

The `++` and `--` operators add `current_value ± 1` to the history:

```c
int x = 5;     // history: [5], value = 5
x++;           // history: [5, 6], value = 6 (ceil of 5.5)
x++;           // history: [5, 6, 7], value = 6 (ceil of 6.0)
```

### Compound Assignment

Compound operators like `+=` also add to history:

```c
int x = 10;    // history: [10]
x += 5;        // history: [10, 15], value = 13 (ceil of 12.5)
```

## The Chaotic For Loop

The most infamous behavior in Blur is the for loop:

```c
for (int i = 0; i < 10; i++) {
    print(i);
}
```

The loop counter `i` averages itself, causing it to "stick" at each value:

```
i = 0    (1 iteration)
i = 1    (2 iterations - stuck)
i = 2    (7 iterations - stuck longer)
i = 3    (20 iterations)
i = 4    (52 iterations)
...
```

The number of iterations at each value grows exponentially. A simple loop to 10 may never terminate!

**Safety limit:** Regular `for` loops automatically stop after 1000 iterations with a warning. Use `sharp for` for unlimited iterations.

### Escape Hatch: sharp for

To create a loop that behaves normally, use the `sharp` keyword:

```c
sharp for (int i = 0; i < 10; i++) {
    print(i);  // Prints 0, 1, 2, 3, 4, 5, 6, 7, 8, 9
}
```

Variables declared in a `sharp for` header do not average.

## Boolean Averaging

Booleans use a threshold of 0.5:

```c
bool flag = true;   // history: [T], value = true
flag = false;       // history: [T, F], ratio = 0.5 → true
flag = false;       // history: [T, F, F], ratio = 0.33 → false
```

A single `true` among equals is `true`. A single `true` among many `false` values becomes `false`.

## String Blurring

Strings use **position-based** character averaging. Each position in the string has its own history:

```c
string s = "hello";  // positions: h, e, l, l, o
s = " a   ";         // space is no-op, only 'a' at pos 1 adds to history
print(s);            // "hcllo" - pos 1 averages 'e'(101) + 'a'(97) = 99 = 'c'
```

### Space is a No-Op

Spaces in an assigned string do not add to any position's history. This allows partial updates:

```c
string s = "abc";
s = "  x";       // only position 2 gets 'x' added
print(s);        // "abn" - 'c'(99) + 'x'(120) = 109.5 → 110 = 'n'
```

### Case Averaging

Uppercase and lowercase letters have different ASCII values, so they average:

```c
string s = "abc";
s = "ABC";
s = "ABC";
print(s);        // "LMN" - 'a'(97) + 'A'(65)*2 = 76 = 'L', etc.
```

### String Repetition

Use `"str" * n` to add a string to history multiple times (for weighting):

```c
string s = "aaa" * 3;  // 'a' added 3 times to each position
s = "zzz";             // 'z' added once
print(s);              // "hhh" - avg of 'a'*3 + 'z' = 104 = 'h'
```

### blurstr() Function

Blur multiple strings without a variable:

```c
print(blurstr("hello", " a   "));           // "hcllo"
print(blurstr("aaa" * 3, "zzz"));           // "hhh"
```

## Functions

Functions pass parameters **with their history**:

```c
int double_it(int x) {
    return x * 2;
}

int blur() {
    int val = 10;
    val = 20;           // val = 15 (avg of 10, 20)
    int result = double_it(val);  // passes 15
    return 0;
}
```

## Syntax

Blur uses C-style syntax with the following features:

### Types
- `int`, `float`, `bool`, `char`, `string`, `void`

### Operators
- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Increment/Decrement: `++`, `--`
- Compound: `+=`, `-=`, `*=`, `/=`, `%=`
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `&&`, `||`, `!`

### Control Flow
- `if` / `else`
- `while`
- `for` (with averaging)
- `sharp for` (without averaging)

### Built-in Functions
- `print(args...)` - prints values to stdout
- `blurstr(strs...)` - returns the blur of multiple strings
- `get_blur()` - returns the current blur factor (0.0-1.0)

### Entry Point
Programs start at `blur()`, not `main()`:

```c
int blur() {
    print("Hello, Blur!");
    return 0;
}
```

## Arrays

Each array element maintains its own history:

```c
int arr[3] = {1, 2, 3};
arr[0] = 10;    // arr[0] history: [1, 10], value = 6
arr[1]++;       // arr[1] history: [2, 3], value = 3
```

## Example Programs

### Hello World

```c
int blur() {
    print("Hello, World!");
    return 0;
}
```

### Demonstrating Averaging

```c
int blur() {
    int money = 5;
    print("Start:", money);      // 5

    money++;
    print("After ++:", money);   // 6

    money = 100;
    print("After = 100:", money); // 37

    return 0;
}
```

### Counting (The Hard Way)

```c
int blur() {
    print("Attempting to count to 5...");

    for (int i = 0; i < 5; i++) {
        print("i =", i);
    }

    print("(This takes a very long time)");
    return 0;
}
```

### Counting (The Easy Way)

```c
int blur() {
    print("Counting to 10:");

    sharp for (int i = 0; i < 10; i++) {
        print(i);
    }

    return 0;
}
```

## Implementation

The reference implementation is written in Rust and includes:
- Lexer using the `logos` crate
- Recursive descent parser
- Tree-walking interpreter
- Interactive REPL

### Running Blur

```bash
blur program.blur       # Run a program
blur                    # Start the REPL
blur -e "int x = 5;"    # Execute code directly
blur -                  # Read and execute from stdin
blur --help             # Show help
```

Examples:
```bash
blur -e "int x = 5; x++; x = 10; print(x);"
echo "int x = 5; print(x);" | blur -
```

### REPL Commands

- `.help` - Show help
- `.vars` - Show variables and their history
- `.blur [value]` - Show or set blur factor (0.0-1.0)
- `.clear` - Reset interpreter state
- `.load <file>` - Load and run a .blur file (C64 style!)
- `.run [func]` - Run a function (default: blur)
- `.exit` - Quit

## Computational Class

Blur is likely Turing-complete when using `sharp for` loops, as it can simulate standard imperative programs. The behavior of regular `for` loops makes certain computations impractical or impossible to complete in reasonable time.

## External Resources

- [GitHub Repository](https://github.com/choas/blur)

## See Also

- [[C]] - The language Blur is based on
- [[Fractran]] - Another language with unusual arithmetic semantics

[[Category:Languages]]
[[Category:2025]]
[[Category:Implemented]]
[[Category:Turing complete]]
