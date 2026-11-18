# PROTOTYPE: block and layout style (throwaway, never merged)

Answers [Block and layout style](https://github.com/marcelocg/vai/issues/6).

The same program appears in three layouts. Only **layout** is under judgement: block delimiters, statement termination, and call parentheses.
Everything else (`mut self`, `Str`, `|a|` closures, `pat -> expr` arms, `"#{}"` strings, `[..]` vec literals) is a placeholder held **constant** across variants and decided by other tickets.

- `a-ruby-end.vai`: Ruby-style. Keyword opens the block, `end` closes it, no `do` on definitions.
- `b-indent.vai`: significant indentation, no closers.
- `c-elixir-do-end.vai`: Elixir-style. `do … end` on every block.
- `generated.rs`: the Rust that all three lower to.
