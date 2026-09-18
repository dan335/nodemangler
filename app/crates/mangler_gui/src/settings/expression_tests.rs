//! Tests for the numeric-field expression evaluator.

use super::*;

/// `evaluate` must succeed and land within f64 noise of `expected`.
fn assert_eval(text: &str, expected: f64) {
    let got = evaluate(text).unwrap_or_else(|| panic!("{text:?} should evaluate"));
    assert!(
        (got - expected).abs() <= 1e-9 * expected.abs().max(1.0),
        "{text:?}: expected {expected}, got {got}"
    );
}

#[test]
fn plain_numbers_take_the_fast_path_and_match_str_parse() {
    for text in ["12", "3.5", ".5", "10.", "-4", "+4", "1e3", "1.5E-2", "  7  "] {
        assert_eq!(evaluate(text), text.trim().parse::<f64>().ok(), "{text:?}");
    }
}

#[test]
fn non_finite_plain_numbers_are_rejected() {
    for text in ["inf", "-inf", "infinity", "nan", "NaN"] {
        assert_eq!(evaluate(text), None, "{text:?}");
    }
}

#[test]
fn the_motivating_example() {
    assert_eval("3.5*300", 1050.0);
}

#[test]
fn precedence_and_associativity() {
    assert_eval("2+3*4", 14.0);
    assert_eval("2*3+4", 10.0);
    assert_eval("2*3^2", 18.0);
    assert_eval("2^3^2", 512.0);
    assert_eval("-2^2", -4.0);
    assert_eval("(-2)^2", 4.0);
    assert_eval("2^-1", 0.5);
    assert_eval("10-4-3", 3.0);
    assert_eval("64/4/2", 8.0);
    assert_eval("7%3", 1.0);
    assert_eval("1+2*3-4/2", 5.0);
}

#[test]
fn parentheses_and_nesting() {
    assert_eval("(2+3)*4", 20.0);
    assert_eval("((1+1)*(2+2))", 8.0);
    assert_eval("2*(3+(4*(5-1)))", 38.0);
}

#[test]
fn unary_chains() {
    assert_eval("--3", 3.0);
    assert_eval("-+-3", 3.0);
    assert_eval("2*-3", -6.0);
    assert_eval("2--3", 5.0);
}

#[test]
fn numbers_with_exponents_inside_expressions() {
    assert_eval("1e3*2", 2000.0);
    assert_eval("1.5E-2*100", 1.5);
    assert_eval(".5+.25", 0.75);
    // An `e` with no digits after it is the constant, not an exponent.
    assert_eval("2*e", 2.0 * E);
}

#[test]
fn whitespace_is_ignored_everywhere() {
    assert_eval("  3.5 * 300  ", 1050.0);
    assert_eval("max ( 1 , 2 )", 2.0);
    assert_eval("- 2", -2.0);
}

#[test]
fn constants() {
    assert_eval("pi", PI);
    assert_eval("PI", PI);
    assert_eval("tau", TAU);
    assert_eval("e", E);
    assert_eval("2*pi", TAU);
}

#[test]
fn one_argument_functions() {
    assert_eval("sqrt(16)", 4.0);
    assert_eval("SQRT(2)*SQRT(2)", 2.0);
    assert_eval("abs(-3)", 3.0);
    assert_eval("sin(0)", 0.0);
    assert_eval("cos(0)", 1.0);
    assert_eval("tan(0)", 0.0);
    assert_eval("asin(1)", PI / 2.0);
    assert_eval("acos(1)", 0.0);
    assert_eval("atan(1)", PI / 4.0);
    assert_eval("floor(2.7)", 2.0);
    assert_eval("ceil(2.1)", 3.0);
    assert_eval("round(2.5)", 3.0);
    assert_eval("trunc(-2.7)", -2.0);
    assert_eval("ln(e)", 1.0);
    assert_eval("log(1000)", 3.0);
    assert_eval("log10(100)", 2.0);
    assert_eval("log2(8)", 3.0);
    assert_eval("exp(0)", 1.0);
    assert_eval("rad(180)", PI);
    assert_eval("deg(pi)", 180.0);
}

#[test]
fn two_and_three_argument_functions() {
    assert_eval("min(3, 7)", 3.0);
    assert_eval("max(3, 7)", 7.0);
    assert_eval("pow(2, 10)", 1024.0);
    assert_eval("atan2(1, 1)", PI / 4.0);
    assert_eval("log(8, 2)", 3.0);
    assert_eval("clamp(15, 0, 10)", 10.0);
    assert_eval("clamp(-5, 0, 10)", 0.0);
    assert_eval("clamp(5, 0, 10)", 5.0);
}

#[test]
fn function_arguments_are_full_expressions() {
    assert_eval("max(1+1, 2*0.5)", 2.0);
    assert_eval("sqrt(3^2+4^2)", 5.0);
    assert_eval("min(max(1,2), max(3,4))", 2.0);
}

#[test]
fn wrong_argument_counts_are_rejected() {
    for text in [
        "sqrt()",
        "sqrt(1,2)",
        "min(1)",
        "min(1,2,3)",
        "clamp(1,2)",
        "clamp(1,2,3,4)",
        "pi()",
        "log()",
    ] {
        assert_eq!(evaluate(text), None, "{text:?}");
    }
}

#[test]
fn inverted_clamp_range_is_rejected_not_a_panic() {
    assert_eq!(evaluate("clamp(5, 10, 0)"), None);
}

#[test]
fn malformed_input_is_rejected() {
    for text in [
        "",
        "   ",
        "3.5*",
        "*3",
        "2 3",
        "(2+3",
        "2+3)",
        "()",
        "2 +* 3",
        "foo",
        "foo(1)",
        "2^",
        "1,2",
        "3..5",
        "1e",
        "1e+",
        "3.5\u{00d7}2",
        "$5",
    ] {
        assert_eq!(evaluate(text), None, "{text:?}");
    }
}

#[test]
fn non_finite_results_are_rejected() {
    assert_eq!(evaluate("1/0"), None);
    assert_eq!(evaluate("-1/0"), None);
    assert_eq!(evaluate("0/0"), None);
    assert_eq!(evaluate("sqrt(-1)"), None);
    assert_eq!(evaluate("ln(0)"), None);
    assert_eq!(evaluate("10^400"), None);
}

#[test]
fn excessive_nesting_is_rejected_rather_than_overflowing() {
    let deep = "(".repeat(MAX_DEPTH + 1) + "1" + &")".repeat(MAX_DEPTH + 1);
    assert_eq!(evaluate(&deep), None);
    let ok = "(".repeat(MAX_DEPTH) + "1" + &")".repeat(MAX_DEPTH);
    assert_eval(&ok, 1.0);
}

#[test]
fn integer_style_use_rounds_as_the_panel_expects() {
    // The panel maps Integer inputs through `f64::round`; make sure the raw
    // value is the exact quotient so that rounding lands on 4, not 3.
    assert_eval("7/2", 3.5);
    assert_eq!(evaluate("7/2").map(f64::round), Some(4.0));
}
