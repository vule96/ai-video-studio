/// Cleans text extracted from a PDF: re-joins hyphenated words, collapses spaces, drops lone page numbers.
pub fn clean_text(text: &str) -> String {
    let text = text.replace('\r', "\n").replace('\u{a0}', " ");
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0;
    let mut lines = text.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).peekable();
    while let Some(line) = lines.next() {
        if line.is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                out.push('\n');
            }
            continue;
        }
        if line.len() <= 4 && line.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        blank_run = 0;
        let joins_next = line.ends_with('-') && lines.peek().is_some_and(|n| n.starts_with(char::is_alphabetic));
        if joins_next {
            out.push_str(&line[..line.len() - 1]);
        } else {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out.trim().to_owned()
}

/// Splits on paragraph boundaries into chunks of roughly `size` characters (never splits inside a char).
pub fn chunk_text(text: &str, size: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut cur = String::new();
    for para in text.split("\n\n").map(str::trim).filter(|p| !p.is_empty()) {
        if !cur.is_empty() && cur.chars().count() + para.chars().count() + 2 > size {
            chunks.push(std::mem::take(&mut cur));
        }
        let chars: Vec<char> = para.chars().collect();
        // A single paragraph longer than `size` is cut into pieces.
        for piece in chars.chunks(size) {
            if !cur.is_empty() {
                cur.push_str("\n\n");
            }
            cur.extend(piece);
            if piece.len() == size {
                chunks.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        chunks.push(cur);
    }
    chunks
}

/// Lowercase ASCII id for characters and file names ("Đông Nhi" → "dong_nhi").
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().flat_map(fold_vietnamese) {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').chars().take(24).collect::<String>();
    if out.is_empty() { "x".into() } else { out }
}

/// Strips Vietnamese diacritics without pulling in a Unicode normalization crate.
fn fold_vietnamese(c: char) -> Option<char> {
    const GROUPS: [(&str, char); 14] = [
        ("àáảãạăằắẳẵặâầấẩẫậ", 'a'),
        ("ÀÁẢÃẠĂẰẮẲẴẶÂẦẤẨẪẬ", 'A'),
        ("èéẻẽẹêềếểễệ", 'e'),
        ("ÈÉẺẼẸÊỀẾỂỄỆ", 'E'),
        ("ìíỉĩị", 'i'),
        ("ÌÍỈĨỊ", 'I'),
        ("òóỏõọôồốổỗộơờớởỡợ", 'o'),
        ("ÒÓỎÕỌÔỒỐỔỖỘƠỜỚỞỠỢ", 'O'),
        ("ùúủũụưừứửữự", 'u'),
        ("ÙÚỦŨỤƯỪỨỬỮỰ", 'U'),
        ("ỳýỷỹỵ", 'y'),
        ("ỲÝỶỸỴ", 'Y'),
        ("đ", 'd'),
        ("Đ", 'D'),
    ];
    if c.is_ascii() {
        return Some(c);
    }
    Some(GROUPS.iter().find(|(set, _)| set.contains(c)).map_or(' ', |&(_, base)| base))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_pdf_artifacts() {
        let raw = "Một đoạn văn bị ngắt dòng ở giữa từ trans-\nformer và   nhiều   khoảng trắng.\n\n\n\n12\nĐoạn hai.";
        assert_eq!(clean_text(raw), "Một đoạn văn bị ngắt dòng ở giữa từ transformer và nhiều khoảng trắng.\n\nĐoạn hai.");
    }

    #[test]
    fn chunks_respect_size_and_keep_all_text() {
        let text = format!("{}\n\n{}\n\n{}", "a".repeat(30), "b".repeat(30), "c".repeat(100));
        let chunks = chunk_text(&text, 40);
        assert!(chunks.iter().all(|c| c.chars().count() <= 40), "{chunks:?}");
        let joined: String = chunks.concat().chars().filter(char::is_ascii_alphabetic).collect();
        assert_eq!(joined.len(), 160);
    }

    #[test]
    fn chunks_multibyte_text_without_panicking() {
        assert_eq!(chunk_text(&"ữ".repeat(10), 3).len(), 4);
    }

    #[test]
    fn slugs_vietnamese_names() {
        assert_eq!(slug("Đông Nhi"), "dong_nhi");
        assert_eq!(slug("  Ông Già Noel!! "), "ong_gia_noel");
        assert_eq!(slug("???"), "x");
    }
}
