# Forge

Forge is a dynamically-typed language written in Rust. It is inspired by JavaScript, Rust, Python and [Rhai](https://github.com/jonathandturner/rhai).
In the future, you'll be able to use Forge as a general-purpose lightweight scripting language in your applications.

[**You can try out Forge in your browser here!**](https://forge.jsbarretto.com)

## Example

```py
# A function to square numbers
var square = |x| {
	return x * x;
};

var n = input "How many squares? ";

# Create a list of squares
var squares = [];
for x in 1..n + 1 {
	squares += square(x);
}

# Iterate and print squares
for square in squares {
	print square;
}
```

## Goals

- Simple, familiar syntax
- Lightweight, quick to parse
- Moderately fast execution speeds
- Well-considered, 'common-sense' design
- Useful, informative error messages
- Easy to build into an existing codebase
- Python-like REPL prompt

## Usage

Using Forge is similar in principle to using Python.
Once compiled, running the shell or executing scripts with the interpreter is trivial.
You'll need to compile the `cli/` crate to gain access to the interpreter binary.

To access the REPL shell, run:

```
$ forge
```

To execute a script, run:

```
$ forge my_script.fg
```

## Roadmap

- [x] Numbers, strings and booleans
- [x] Arithmetic operators *`+`, `-`, `*`, `/`, `%`*
- [x] Logical operators *`and`, `or`, `xor`, `==`, `!=`, `!`, `<`, `<=`, `>`, `>=`*
- [x] `if`/`else` statements
- [x] `while` and `for` statements
- [x] Assignment operators *`=`, `+=`, `-=`, `*=`, `/=`, `%=`*
- [x] Scoped variable declaration
- [x] Function objects
- [x] Function calling
- [x] Rust-to-Forge object interface
- [x] Rust-to-Forge type coercion
- [x] Rust callbacks *Only Rust closures with no arguments or functions are currently supported*
- [x] Iterators
- [x] Rust-to-Forge iterators
- [x] Lists
- [x] List splicing
- [x] Indexing and ranges
- [x] `clone` and `mirror` operators
- [x] Lvalues vs rvalues
- [x] Maps
- [x] Map construction
[ [ ] Map iteration
- [ ] Structures
- [ ] Enums
- [ ] Objects
- [ ] Modules as objects
- [ ] Scoped constants
- [ ] C-based FFI for non-Rust integration
- [ ] AST optimisation
- [ ] Bytecode generation
- [ ] Bytecode interpretation
- [ ] LLVM-driven recompilation

## Some Syntax Examples

List splicing

```
>> var my_list = [0, 1, 2, 3];
>> my_list[1..2]
[1]
>> my_list[1..3] = ["this", "is", "a", "list", "splice"];
>> my_list
[0, this, is, a, list, splice, 3]
```

String splicing

```
>> "Hello, world!"[7..12]
world
>> var test = "An apple is what I am eating";
>> test[3..8] = "pear";
>> test
An pear is what I am eating
>>
```

## Design

### Types

Forge has several distinct types:

- Number *64-bit, floating-point*
- String *unicode-compliant*
- Char *unicode-compliant*
- Boolean
- Range
- Function
- List
- Map
- Object *Currently unimplemented*
- Custom *Used to call to and from Rust*
- Null

### Things To Do

- Investigate design features that would make the dynamic type system easier to optimise

### Interpreter

Currently, Forge is only implemented as an AST-walking interpreter.
In the future, I aim to generate more efficient low-level bytecode for the language.
I also aim to implement many a variety of optimisations throughout the compilation process.

### Error Messages

Forge aims to produce the most useful, informative and intelligence error messages it can.
Errors can be emitted at compile-time or run-time. Below are a few examples.

Parser errors:

```
[ERROR] Parsing error at 1:45...
   ...while parsing if-else statement...
   ...while parsing print statement...
        1| var x = 1; if x > 2 { print "Hello, world!" oops; }
         |                                             ^^^^
   Expected ';', found identifier 'oops'.
```

Runtime errors:

```
[ERROR] Runtime error at 1:21...
        1| var p = true; while p { print "On the next iteration, p will be null"; p = null; }
         |                     ^
   Cannot determine the truthiness of value of type 'null'. Did you mean for this to be a bool?
```

Runtime errors that produce error messages that reference code written during the previous declaration of a function object:

```
[ERROR] Runtime error at 1:10...
        1| var say_hello = || { print "Hello, world!"; };
         |                 ^^
        1| say_hello(1); # Wrong number of parameters
         |          ^^^
   Tried to call a function with the wrong number of parameters. Expected 0, found 1.
```
