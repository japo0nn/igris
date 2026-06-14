use std::fmt::Write;

pub fn render_markdown(text: &str) -> String {
    let mut out = String::new();
    let mut in_code_block = false;
    let mut code_lines: Vec<String> = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            if in_code_block {
                if !code_lines.is_empty() {
                    let _ = write!(out, "\x1b[38;5;244m{}\x1b[0m", code_lines.join("\n"));
                    code_lines.clear();
                }
                out.push('\n');
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }
        if in_code_block {
            code_lines.push(line.to_string());
            continue;
        }
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let level = trimmed.bytes().take_while(|&b| b == b'#').count();
            let rest = trimmed[level..].trim();
            let color = match level {
                1 => "\x1b[1;36m",
                2 => "\x1b[1;34m",
                3 => "\x1b[1;33m",
                _ => "\x1b[1;37m",
            };
            let _ = writeln!(out, "{}{}\x1b[0m", color, render_inline(rest));
        } else if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
        {
            let rest = trimmed[2..].trim();
            let _ = writeln!(out, " \x1b[33m\u{2022}\x1b[0m {}", render_inline(rest));
        } else if let Some(n) = parse_numbered(trimmed) {
            let rest = &trimmed[(n.1 + 2)..].trim();
            let _ = writeln!(out, " \x1b[33m{}.\x1b[0m {}", n.0, render_inline(rest));
        } else {
            let _ = writeln!(out, "{}", render_inline(line));
        }
    }
    if !code_lines.is_empty() {
        let _ = write!(out, "\x1b[38;5;244m{}\x1b[0m", code_lines.join("\n"));
    }
    out
}

fn parse_numbered(trimmed: &str) -> Option<(u32, usize)> {
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    let mut n = 0u32;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        n = n * 10 + (bytes[i] - b'0') as u32;
        i += 1;
    }
    if i > 0 && i < bytes.len() && bytes[i] == b'.' && bytes.get(i + 1) == Some(&b' ') {
        Some((n, i))
    } else {
        None
    }
}

fn render_inline(text: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            out.push(chars[i]);
            out.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if chars[i] == '`' {
            let mut code = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '`' {
                code.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            let _ = write!(out, "\x1b[36m{}\x1b[0m", code);
        } else if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            let mut bold = String::new();
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '*') {
                bold.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() {
                i += 2;
            }
            let _ = write!(out, "\x1b[1m{}\x1b[22m", render_inline(&bold));
        } else if chars[i] == '*' && (i + 1 >= chars.len() || chars[i + 1] != '*') {
            let mut italic = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '*' {
                italic.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            let _ = write!(out, "\x1b[3m{}\x1b[23m", render_inline(&italic));
        } else if chars[i] == '[' {
            let mut link_text = String::new();
            i += 1;
            while i < chars.len() && chars[i] != ']' {
                link_text.push(chars[i]);
                i += 1;
            }
            if i + 1 < chars.len() && chars[i] == ']' && chars[i + 1] == '(' {
                let mut url = String::new();
                i += 2;
                while i < chars.len() && chars[i] != ')' {
                    url.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() {
                    i += 1;
                }
                let _ = write!(out, "\x1b[4;34m{}\x1b[0m", render_inline(&link_text));
            } else {
                out.push('[');
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}
