//! Crate with macro implementations of `expression_format`.
//!
//! A separete crate is required to test procedural macros.

extern crate proc_macro;

use core::str::CharIndices;
use proc_macro::TokenStream;
use regex::Regex;
use std::ops::Range;

// =====================================================================
// public
// =====================================================================

#[proc_macro]
pub fn ex_format(item: TokenStream) -> TokenStream {
    ex_impl("format", &item.to_string()).parse().unwrap()
}

#[proc_macro]
pub fn ex_print(item: TokenStream) -> TokenStream {
    ex_impl("print", &item.to_string()).parse().unwrap()
}

#[proc_macro]
pub fn ex_println(item: TokenStream) -> TokenStream {
    ex_impl("println", &item.to_string()).parse().unwrap()
}

#[proc_macro]
pub fn ex_eprint(item: TokenStream) -> TokenStream {
    ex_impl("eprint", &item.to_string()).parse().unwrap()
}

#[proc_macro]
pub fn ex_eprintln(item: TokenStream) -> TokenStream {
    ex_impl("eprintln", &item.to_string()).parse().unwrap()
}

// =====================================================================
// private
// =====================================================================

fn ex_impl(func: &str, arg: &str) -> String {
    let mod_regex = Regex::new(r#"^:(?:(?:.?[<\^>])?[\+\-]?#?0?\d*(?:\.\d+)?(?:[oxXpbeE]?\??))"#).unwrap();
    let mut ex_fmt = String::with_capacity(arg.len());
    let mut ex_args = String::with_capacity(arg.len());
    let mut search_index = 0;

    while let Some(expr_range) = range_in_brackets(&arg, search_index, &mod_regex) {
        ex_fmt.push_str(&arg[search_index..expr_range.start]);
        search_index = expr_range.end;
        ex_args.push(',');
        ex_args.push_str(&arg[expr_range]);
    }

    ex_fmt.push_str(&arg[search_index..]);

    format!("{}!({}{})", func, ex_fmt, ex_args)
}

type Iter<'a> = CharIndices<'a>;


fn range_in_brackets(arg: &str, start_index: usize, specs: &Regex) -> Option<Range<usize>> {
    let mut start = match find_expr_start(&mut arg[start_index..].char_indices()) {
        Some(i) => start_index + i,
        None => return None,
    };

    if let Some(modifier) = specs.find(&arg[start..]) {
        start += modifier.end(); 
    }

    if let Some(end) = find_expr_end(&mut arg[start..].char_indices()) {
        return Some(start..(start + end));
    }

    None
}

fn find_expr_start(iter: &mut Iter) -> Option<usize> {
    let mut prev_c = 0 as char;
    for (i, c) in iter {
        if prev_c == '{' {
            if c != '{' {
                return Some(i)
            } 
            prev_c = 0 as char;
        } else {
            prev_c = c;
        }
    }

    None
}

// counts opening and closing brackets and recognizes all the constructs
// where they should be ignored
fn find_expr_end(iter: &mut Iter) -> Option<usize> {
    let mut prev_c = 0 as char;
    let mut depth = 1;
    while let Some((i, c)) = iter.next() {
        prev_c = match c {
            '{' => {
                depth += 1;
                0 as char
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
                0 as char
            }
            'r' => try_raw_string(iter, prev_c),
            '"' => do_string(iter),
            '/' => try_line_comment(iter, prev_c),
            '*' => try_block_comment(iter, prev_c),
            '\'' => try_char(iter),
            _ => c,
        };
    }

    None
}

fn try_raw_string(iter: &mut Iter, prev_c: char) -> char {
    if prev_c.is_ascii_alphanumeric() || prev_c == '_' {
        return 'r';
    }
    let mut depth = 0;
    let mut iter_copy = iter.clone();
    let mut prev_c = 'r';
    while let Some((_, c)) = iter_copy.next() {
        if c == '"' {
            iter.next();
            do_raw_string(iter, depth);
            break;
        } else if c == '#' {
            iter.next();
            depth += 1;
        } else {
            return prev_c;
        }

        prev_c = c;
    }

    0 as char
}

fn do_raw_string(iter: &mut Iter, depth: usize) {
    let mut depth_counter = depth;
    let mut prev_c = 0 as char;
    for (_, c) in iter {
        if c == '"' && depth == 0 {
            return;
        } else if c == '#' {
            if prev_c == '"' || depth_counter != depth {
                depth_counter -= 1;
                if depth_counter == 0 {
                    return;
                }
            }
        } else if depth_counter != depth {
            depth_counter = depth;
        }

        prev_c = c;
    }
}

fn do_string(iter: &mut Iter) -> char {
    let mut prev_c = 0 as char;
    for (_, c) in iter {
        if c == '"' && prev_c != '\\' {
            break;
        }
        prev_c = c;
    }

    return 0 as char;
}

fn try_char(iter: &mut Iter) -> char {
    if let Some((_, c)) = iter.next() {
        if c.is_ascii_alphabetic() || c == '_' {
            let iter_copy = iter.clone();

            for (_, c) in iter_copy {
                if !(c.is_ascii_alphanumeric() || c == '_') {
                    if c == '\'' {
                        iter.next();
                    }

                    return 0 as char;
                }

                iter.next();
            }
        } else {
            let mut prev_c = c;
            for (_, c) in iter {
                if c == '\'' && prev_c != '\\' {
                    return 0 as char;
                }
                prev_c = c;
            }
        }
    }

    0 as char
}

fn try_line_comment(iter: &mut Iter, prev_c: char) -> char {
    if prev_c == '/' {
        for (_, c) in iter {
            if c == '\n' {
                break;
            }
        }

        return 0 as char;
    }

    '/'
}

fn try_block_comment(iter: &mut Iter, prev_c: char) -> char {
    if prev_c == '/' {
        let mut prev_c = 0 as char;
        let mut depth = 1;
        for (_, c) in iter {
            if prev_c == '/' && c == '*' {
                depth += 1;
                prev_c = 0 as char;
            } else if prev_c == '*' && c == '/' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                prev_c = 0 as char;
            } else {
                prev_c = c;
            }
        }

        return 0 as char;
    }

    '*'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unbalanced_bracket_in_string_with_escape() {
        test_helper(r#""lorem {"{\"{"} ipsum""#, r#""lorem {} ipsum","{\"{""#);
    }

    #[test]
    fn test_unbalanced_bracket_in_char() {
        test_helper(r#""lorem {'{'} ipsum""#, r#""lorem {} ipsum",'{'"#);
    }

    #[test]
    fn test_unbalanced_bracket_in_string() {
        test_helper(
            r#""lorem {"{dolor"} ipsum""#,
            r#""lorem {} ipsum","{dolor""#,
        );
    }

    #[test]
    fn test_struct_create() {
        test_helper(
            r#""lorem {Dolor{sit:"amet"}} ipsum""#,
            r#""lorem {} ipsum",Dolor{sit:"amet"}"#,
        );
    }

    #[test]
    fn test_literal_string() {
        test_helper(
            r#""lorem {"dolor sit amet"} ipsum""#,
            r#""lorem {} ipsum","dolor sit amet""#,
        );
    }

    #[test]
    fn test_consecutive_formats() {
        test_helper(r#""{lorem}{ipsum}""#, r#""{}{}",lorem,ipsum"#);
    }

    #[test]
    fn test_escaped_open_brackets() {
        test_helper(r#""lorem {{ ipsum""#, r#""lorem {{ ipsum""#);
    }

    #[test]
    fn test_escaped_close_brackets() {
        test_helper(r#""lorem }} ipsum""#, r#""lorem }} ipsum""#);
    }

    #[test]
    fn test_escaped_brackets() {
        test_helper(r#""{{}}""#, r#""{{}}""#);
    }

    #[test]
    fn test_no_format() {
        test_helper(r#""lorem ipsum""#, r#""lorem ipsum""#);
    }

    #[test]
    fn test_no_arg() {
        test_helper("", "");
    }

    #[test]
    fn test_empty_str_arg() {
        test_helper(r#""""#, r#""""#);
    }

    #[test]
    fn test_empty_marker() {
        test_helper(r#""lorem {} ipsum""#, r#""lorem {} ipsum","#);
    }

    #[test]
    fn test_literal_number() {
        test_helper(r#""lorem {123} ipsum""#, r#""lorem {} ipsum",123"#);
    }

    #[test]
    fn test_one_argument() {
        test_helper(r#""lorem {dolor} ipsum""#, r#""lorem {} ipsum",dolor"#);
    }

    #[test]
    fn test_multiple_same_argument() {
        test_helper(
            r#""lorem {dolor} ipsum {dolor} sit {dolor} amet""#,
            r#""lorem {} ipsum {} sit {} amet",dolor,dolor,dolor"#,
        );
    }

    #[test]
    fn test_multiple_arguments() {
        test_helper(
            r#""lorem {ipsum} dolor {sit} amet, {consectetur} adipiscing""#,
            r#""lorem {} dolor {} amet, {} adipiscing",ipsum,sit,consectetur"#,
        );
    }

    #[test]
    fn test_nested_argument() {
        test_helper(
            r#""lorem {dolor.sit} ipsum""#,
            r#""lorem {} ipsum",dolor.sit"#,
        );
    }

    #[test]
    fn test_array_argument() {
        test_helper(
            r#""lorem {dolor[0]} ipsum""#,
            r#""lorem {} ipsum",dolor[0]"#,
        );
    }

    #[test]
    fn test_function_argument() {
        test_helper(
            r#""lorem {dolor(arg)} ipsum""#,
            r#""lorem {} ipsum",dolor(arg)"#,
        );
    }

    #[test]
    fn test_debug_argument() {
        test_helper(r#""lorem {:?dolor} ipsum""#, r#""lorem {:?} ipsum",dolor"#);
    }

    #[test]
    fn test_mixed_empty_argument() {
        test_helper(
            r#""lorem {dolor} ipsum {} sit""#,
            r#""lorem {} ipsum {} sit",dolor,"#,
        );
    }

    #[test]
    fn test_quote_outside_expression() {
        test_helper(r#""lorem "{"ipsum"}"""#, r#""lorem "{}"","ipsum""#);
    }

    #[test]
    fn test_raw_string() {
        test_helper(
            r###""{r##""{lorem}"#"{""##} {ipsum}""###,
            r###""{} {}",r##""{lorem}"#"{""##,ipsum"###,
        );
    }

    #[test]
    fn test_line_comment() {
        test_helper(
            r#""{ {// lorem ipsum { "
"dolor"} }""#,
            r#""{}", {// lorem ipsum { "
"dolor"} "#,
        );
    }

    #[test]
    fn test_block_comment() {
        test_helper(r#""{/*lorem { ipsum*/10}""#, r#""{}",/*lorem { ipsum*/10"#);
    }

    #[test]
    fn test_nested_block_comment() {
        test_helper(
            r#""lorem {/*/*inside comment*/still inside comment*/"ipsum"}""#,
            r#""lorem {}",/*/*inside comment*/still inside comment*/"ipsum""#,
        );
    }

    #[test]
    fn test_format_width() {
        test_helper(r#""{:04 42}""#, r#""{:04}", 42"#);
    }

    #[test]
    fn test_format_alignment_with_char() {
        test_helper(r#""{:'>10 "test"}""#, r#""{:'>10}", "test""#);
    }

    fn test_helper(in_arg: &str, out_arg: &str) {
        let expected = format!("format!({})", out_arg);
        assert_eq!(ex_impl("format", in_arg), expected);
    }
}
