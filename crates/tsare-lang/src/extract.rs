//! Reading a definition out of a rendered page.
//!
//! A prose chapter says what a word means in the two places a machine can find without guessing:
//! a section heading, and a table row that pairs a term with its gloss.  Which column holds the
//! term is not something this file decides -- the caller names the header ("术语", "名称", "组件")
//! and the extractor takes that column, or, with no header named, the first cell when it is the
//! only bold thing in the row.  Those together are a `defines` arrow: the section, the term, and
//! the line that states it.
//!
//! Nothing else in the text is read.  A paragraph that defines a word in passing stays unread,
//! because the alternative is a rule that guesses, and a guess in the extractor is a claim the
//! checker can never refuse.  A table whose header names no term column is read as prose and
//! contributes nothing -- the omission is visible in the count, which is the point.

/// One term the text defines, and where it does it.
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub term: String,
    /// The nearest heading above the row: what the definition is a definition *of*.
    pub section: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
}

/// The header words a vocabulary document names.
///
/// A term column is a *preference*, not a commitment: nothing in a page says "this column is a
/// table of terms", so the person doing the reading says it.  A `note` with `tag: preference` is
/// exactly that -- held without owing anyone a proof -- and it is written in the same language as
/// everything else, so `check` still reads it.
pub fn vocabulary(text: &str, file: &str) -> Result<Vec<String>, String> {
    let decls = crate::syntax::parse(text).map_err(|error| format!("{file}: {error}"))?;
    let mut out = Vec::new();
    for decl in decls.iter().filter(|decl| decl.kind == "note") {
        if crate::program::f_id(decl, "tag") != Some("preference") {
            continue;
        }
        if let Some(word) = crate::program::f_str(decl, "text") {
            let word = word.trim();
            if !word.is_empty() {
                out.push(word.to_string());
            }
        }
    }
    Ok(out)
}

/// Every definition the page states, in reading order.  `terms` are the header words that name a
/// term column; empty means "only rows whose first cell is the single bold phrase".
pub fn definitions(text: &str, file: &str, terms: &[&str]) -> Vec<Definition> {
    let mut out: Vec<Definition> = Vec::new();
    let mut section = String::new();
    let mut column: Option<usize> = None;
    for (index, line) in text.lines().enumerate() {
        if let Some(heading) = heading(line) {
            section = heading;
            column = None;
            continue;
        }
        if let Some(headers) = headers(line) {
            column = (!terms.is_empty())
                .then(|| term_column(&headers, terms))
                .flatten();
            continue;
        }
        if line.contains("</table>") {
            column = None;
        }
        let found = bold_term_row(line).or_else(|| {
            let index = column?;
            cells(line)
                .get(index)
                .filter(|(text, _)| !text.is_empty())
                .cloned()
        });
        if let Some((term, col)) = found {
            out.push(Definition {
                term,
                section: section.clone(),
                file: file.to_string(),
                line: index + 1,
                col,
            });
        }
    }
    out
}

fn unescape(fragment: &str) -> String {
    fragment
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

fn text_of(fragment: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for character in fragment.chars() {
        match character {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(character),
            _ => {}
        }
    }
    unescape(&out).trim().to_string()
}

fn heading(line: &str) -> Option<String> {
    // any level: a chapter uses h2 for its parts and h3 for the section that defines a term
    let mut rest = line;
    loop {
        let open = rest.find("<h")?;
        let level = rest[open + 2..].chars().next()?;
        if !level.is_ascii_digit() {
            rest = &rest[open + 2..];
            continue;
        }
        let start = rest[open..].find('>')? + open + 1;
        let end = rest[start..].find("</h")? + start;
        let text = text_of(&rest[start..end]);
        return (!text.is_empty()).then_some(text);
    }
}

/// The header words of a table, when the line opens one.
fn headers(line: &str) -> Option<Vec<String>> {
    if !line.contains("<th") {
        return None;
    }
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find("<th") {
        let start = rest[open..].find('>')? + open + 1;
        let end = rest[start..].find("</th>")? + start;
        out.push(text_of(&rest[start..end]));
        rest = &rest[end + 5..];
    }
    (!out.is_empty()).then_some(out)
}

/// Which column the caller's header words name, if any.
fn term_column(headers: &[String], terms: &[&str]) -> Option<usize> {
    headers
        .iter()
        .position(|header| terms.iter().any(|term| term == header))
}

/// The cells of one table row, with the 1-based column each cell's text starts at.
fn cells(line: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find("<td") {
        let Some(start) = rest[open..].find('>').map(|end| end + open + 1) else {
            break;
        };
        let Some(end) = rest[start..].find("</td>").map(|end| end + start) else {
            break;
        };
        out.push((text_of(&rest[start..end]), start + 1));
        rest = &rest[end + 5..];
    }
    out
}

/// A row shaped `<tr><td><strong>TERM</strong></td><td>GLOSS...`, which is how the chapters pair a
/// term with its definition.  The column is 1-based and points at the first character of the term.
fn bold_term_row(line: &str) -> Option<(String, usize)> {
    let open = line.find("<tr><td><strong>")? + "<tr><td><strong>".len();
    let end = line[open..].find("</strong>")? + open;
    let after = &line[end..];
    if !after.starts_with("</strong></td><td>") {
        return None;
    }
    let term = text_of(&line[open..end]);
    if term.is_empty() {
        return None;
    }
    Some((term, open + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "\
<h2>框.2 变换平面</h2>
<div class=\"table-wrap\"><table><thead><tr><th>平面</th><th>它承载什么</th></tr></thead><tbody>
<tr><td><strong>结构平面</strong></td><td>对象与视图：树、图、格。</td></tr>
<tr><td><strong>证据平面</strong></td><td>验证内核与三态。</td></tr>
</tbody></table></div>
<h3>核.1 八条不变量</h3>
<tr><td><strong>不变量 3</strong></td><td>每个结论携带三态。</td></tr>
<p>这一段<strong>不是</strong>定义。</p>
";

    const PLAIN: &str = "\
<h2>B.1 能力树</h2>
<table><thead><tr><th>层次</th><th>名称</th><th>含义</th></tr></thead><tbody>
<tr><td>核心</td><td>思维代码</td><td>能指与所指的纪律</td></tr>
<tr><td>地基</td><td>生成压</td><td>有界压缩</td></tr>
</tbody></table>
";

    #[test]
    fn a_bold_term_in_the_first_cell_is_a_definition() {
        let found = definitions(PAGE, "page.html", &[]);
        assert_eq!(found.len(), 3, "{found:#?}");
        assert_eq!(found[0].term, "结构平面");
        assert_eq!(found[0].section, "框.2 变换平面");
        assert_eq!(found[0].line, 3);
        assert_eq!(found[2].section, "核.1 八条不变量");
    }

    #[test]
    fn a_bold_word_inside_a_paragraph_is_not() {
        let found = definitions("<p>这一段<strong>不是</strong>定义。</p>", "page.html", &[]);
        assert!(found.is_empty(), "{found:#?}");
    }

    #[test]
    fn a_declared_header_names_the_term_column() {
        assert!(
            definitions(PLAIN, "page.html", &[]).is_empty(),
            "unread without a header"
        );
        let found = definitions(PLAIN, "page.html", &["名称"]);
        assert_eq!(found.len(), 2, "{found:#?}");
        assert_eq!(found[0].term, "思维代码");
        assert_eq!(found[1].term, "生成压");
    }
}
