use super::error::MemoryError;

/// Compress a block of text down to roughly `max_sentences` sentences.
///
/// This is intentionally light-weight to avoid pulling in a tokenizer while
/// still giving us a reasonable summary for progressive disclosure.
pub fn compress_sentences(content: &str, max_sentences: usize) -> String {
    if max_sentences == 0 {
        return String::new();
    }

    let mut out = String::new();
    let mut count = 0usize;

    for sentence in content.split(|c| c == '.' || c == '!' || c == '?') {
        let sentence = sentence.trim();
        if sentence.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str(". ");
        }
        out.push_str(sentence);
        count += 1;
        if count >= max_sentences {
            break;
        }
    }

    if !out.ends_with(['.', '!', '?']) && !out.is_empty() {
        out.push('.');
    }

    out
}

/// Merge the compressed summary with a small tail of the original content
/// to retain recency for workers that only receive minimal context.
pub fn tail_context(content: &str, max_chars: usize) -> Result<String, MemoryError> {
    if max_chars == 0 {
        return Err(MemoryError::invalid("max_chars must be > 0"));
    }

    if content.len() <= max_chars {
        return Ok(content.to_string());
    }

    let tail = &content[content.len() - max_chars..];
    Ok(format!("...[omitted]\n{}", tail))
}
