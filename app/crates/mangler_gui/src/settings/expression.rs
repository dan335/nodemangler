//! Arithmetic expression evaluation for the numeric input boxes.
//!
//! Typing `3.5*300` into a width field should yield `1050`, the way Fusion's
//! and Blender's numeric fields do. `evaluate` is plugged into egui's
//! `DragValue::custom_parser` / `Slider::custom_parser` (see
//! `node_settings_panel.rs::input_value`), which egui calls only when the text
//! edit commits — Enter or focus loss — never per keystroke, so a half-typed
//! `3.5*3` is never committed as `10.5`.
//!
//! Hand-rolled recursive descent over `f64` rather than a crate: the grammar
//! is a dozen lines, the codebase already hand-rolls base64 and version
//! parsing for the same reason, and a parser dependency would be the only
//! thing in the tree pulling in a lexer and an AST just for a text box.
//!
//! Grammar (usual precedence; `^` is right-associative and binds tighter than
//! unary minus, so `-2^2` is `-4` and `2^3^2` is `512`):
//!
//! ```text
//! expr   := term (('+' | '-') term)*
//! term   := unary (('*' | '/' | '%') unary)*
//! unary  := ('-' | '+') unary | power
//! power  := atom ('^' unary)?
//! atom   := number | constant | ident '(' expr (',' expr)* ')' | '(' expr ')'
//! ```
//!
//! Names are case-insensitive. Constants: `pi`, `tau`, `e`. Functions:
//! `sqrt abs sin cos tan asin acos atan atan2 floor ceil round trunc ln log
//! log10 log2 exp rad deg min max pow clamp` (trig in radians; `log(x)` is
//! base 10, `log(x, b)` base `b`).
//!
//! Anything malformed — trailing text, an unbalanced paren, an unknown name,
//! a wrong argument count, a non-finite result such as `1/0` — evaluates to
//! `None`, and egui then leaves the value untouched, exactly as it does today
//! for a typed non-number.

use std::f64::consts::{E, PI, TAU};

/// Deepest nesting of parentheses / function calls accepted. A typed
/// expression never gets anywhere near this; the cap only turns a pathological
/// paste into `None` instead of a stack overflow.
const MAX_DEPTH: usize = 64;

/// Most arguments any function takes (`clamp`).
const MAX_ARGS: usize = 3;

/// Evaluate an arithmetic expression to a finite number, or `None` if the
/// text is not a valid expression or its value is not finite.
///
/// A plain number (anything `str::parse::<f64>` accepts) short-circuits
/// through that same parse, so a field that is only ever typed into as a
/// number behaves bit-identically to egui's default parser.
pub fn evaluate(text: &str) -> Option<f64> {
    let text = text.trim();
    if let Ok(v) = text.parse::<f64>() {
        // `inf` / `nan` parse successfully and must not reach a value.
        return v.is_finite().then_some(v);
    }
    let mut parser = Parser { src: text.as_bytes(), pos: 0, depth: 0 };
    let value = parser.expr()?;
    parser.skip_ws();
    if parser.pos != parser.src.len() {
        // Trailing text the grammar didn't consume (`3.5*`, `2 3`, `4)`).
        return None;
    }
    value.is_finite().then_some(value)
}

/// Cursor over the expression bytes. The grammar is ASCII-only, so byte
/// scanning is exact: any non-ASCII byte simply fails to match anything and
/// surfaces as trailing text.
struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    /// Current paren / call nesting, checked against `MAX_DEPTH`.
    depth: usize,
}

impl Parser<'_> {
    /// Advance past any run of whitespace.
    fn skip_ws(&mut self) {
        while self.src.get(self.pos).is_some_and(u8::is_ascii_whitespace) {
            self.pos += 1;
        }
    }

    /// Next significant byte, skipping whitespace, without consuming it.
    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.src.get(self.pos).copied()
    }

    /// Consume `c` if it is the next significant byte.
    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// `expr := term (('+' | '-') term)*`
    fn expr(&mut self) -> Option<f64> {
        let mut acc = self.term()?;
        loop {
            if self.eat(b'+') {
                acc += self.term()?;
            } else if self.eat(b'-') {
                acc -= self.term()?;
            } else {
                return Some(acc);
            }
        }
    }

    /// `term := unary (('*' | '/' | '%') unary)*`
    fn term(&mut self) -> Option<f64> {
        let mut acc = self.unary()?;
        loop {
            if self.eat(b'*') {
                acc *= self.unary()?;
            } else if self.eat(b'/') {
                acc /= self.unary()?;
            } else if self.eat(b'%') {
                acc %= self.unary()?;
            } else {
                return Some(acc);
            }
        }
    }

    /// `unary := ('-' | '+') unary | power`
    fn unary(&mut self) -> Option<f64> {
        if self.eat(b'-') {
            return self.unary().map(|v| -v);
        }
        if self.eat(b'+') {
            return self.unary();
        }
        self.power()
    }

    /// `power := atom ('^' unary)?` — right-associative, and the exponent is a
    /// `unary` so `2^-1` works and `2^3^2` groups as `2^(3^2)`.
    fn power(&mut self) -> Option<f64> {
        let base = self.atom()?;
        if self.eat(b'^') {
            let exponent = self.unary()?;
            return Some(base.powf(exponent));
        }
        Some(base)
    }

    /// `atom := number | constant | call | '(' expr ')'`
    fn atom(&mut self) -> Option<f64> {
        match self.peek()? {
            b'(' => {
                self.pos += 1;
                let value = self.nested(Self::expr)?;
                self.eat(b')').then_some(value)
            }
            c if c.is_ascii_digit() || c == b'.' => self.number(),
            c if c.is_ascii_alphabetic() || c == b'_' => self.ident(),
            _ => None,
        }
    }

    /// Run `inner` one nesting level deeper, refusing past `MAX_DEPTH`.
    fn nested(&mut self, inner: fn(&mut Self) -> Option<f64>) -> Option<f64> {
        if self.depth >= MAX_DEPTH {
            return None;
        }
        self.depth += 1;
        let result = inner(self);
        self.depth -= 1;
        result
    }

    /// Scan `digits [. digits] [(e|E) [+|-] digits]` and hand the slice to
    /// `str::parse`, which owns the actual numeric conversion (so `1.5E-2`,
    /// `.5` and `10.` all mean what they mean to Rust). An `e` not followed by
    /// digits is left unconsumed — it is the constant `e` or trailing junk,
    /// not an exponent.
    fn number(&mut self) -> Option<f64> {
        let start = self.pos;
        let digit_or_dot = |c: &u8| c.is_ascii_digit() || *c == b'.';
        while self.src.get(self.pos).is_some_and(digit_or_dot) {
            self.pos += 1;
        }
        if matches!(self.src.get(self.pos), Some(b'e' | b'E')) {
            let before_exponent = self.pos;
            self.pos += 1;
            if matches!(self.src.get(self.pos), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if self.src.get(self.pos).is_some_and(u8::is_ascii_digit) {
                while self.src.get(self.pos).is_some_and(u8::is_ascii_digit) {
                    self.pos += 1;
                }
            } else {
                self.pos = before_exponent;
            }
        }
        std::str::from_utf8(&self.src[start..self.pos]).ok()?.parse().ok()
    }

    /// A bare name is a constant; a name followed by `(` is a function call.
    fn ident(&mut self) -> Option<f64> {
        let start = self.pos;
        while self
            .src
            .get(self.pos)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            self.pos += 1;
        }
        let name = std::str::from_utf8(&self.src[start..self.pos])
            .ok()?
            .to_ascii_lowercase();

        if !self.eat(b'(') {
            return match name.as_str() {
                "pi" => Some(PI),
                "tau" => Some(TAU),
                "e" => Some(E),
                _ => None,
            };
        }

        // Arguments land in a fixed array: no function takes more than
        // `MAX_ARGS`, so an over-long list is a user error, not a bigger Vec.
        let mut args = [0.0f64; MAX_ARGS];
        let mut count = 0;
        if !self.eat(b')') {
            loop {
                if count == MAX_ARGS {
                    return None;
                }
                args[count] = self.nested(Self::expr)?;
                count += 1;
                if self.eat(b',') {
                    continue;
                }
                if self.eat(b')') {
                    break;
                }
                return None;
            }
        }
        apply(&name, &args[..count])
    }
}

/// Apply a named function to its (already evaluated) arguments. A wrong
/// argument count is `None`, never a default.
fn apply(name: &str, args: &[f64]) -> Option<f64> {
    let one = |f: fn(f64) -> f64| match args {
        [x] => Some(f(*x)),
        _ => None,
    };
    let two = |f: fn(f64, f64) -> f64| match args {
        [a, b] => Some(f(*a, *b)),
        _ => None,
    };
    match name {
        "sqrt" => one(f64::sqrt),
        "abs" => one(f64::abs),
        "sin" => one(f64::sin),
        "cos" => one(f64::cos),
        "tan" => one(f64::tan),
        "asin" => one(f64::asin),
        "acos" => one(f64::acos),
        "atan" => one(f64::atan),
        "floor" => one(f64::floor),
        "ceil" => one(f64::ceil),
        "round" => one(f64::round),
        "trunc" => one(f64::trunc),
        "ln" => one(f64::ln),
        "log10" => one(f64::log10),
        "log2" => one(f64::log2),
        "exp" => one(f64::exp),
        "rad" => one(f64::to_radians),
        "deg" => one(f64::to_degrees),
        // One argument is base 10 (the common reading); two picks the base.
        "log" => match args {
            [x] => Some(x.log10()),
            [x, base] => Some(x.log(*base)),
            _ => None,
        },
        "atan2" => two(f64::atan2),
        "min" => two(f64::min),
        "max" => two(f64::max),
        "pow" => two(f64::powf),
        // `f64::clamp` panics on an inverted or NaN range; refuse those instead.
        "clamp" => match args {
            [x, lo, hi] if lo <= hi => Some(x.clamp(*lo, *hi)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
#[path = "expression_tests.rs"]
mod tests;
