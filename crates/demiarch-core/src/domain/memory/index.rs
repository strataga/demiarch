use super::error::MemoryError;
use super::timeline::TimelineEntry;

/// Generate a compact index-level summary from a timeline entry.
///
/// The index layer is intentionally tiny so we aggressively truncate while
/// keeping the most informative pieces of text.
pub fn generate_index_summary(timeline: &TimelineEntry) -> Result<String, MemoryError> {
    if timeline.summary.trim().is_empty() {
        return Err(MemoryError::invalid("timeline summary missing"));
    }

    // Start with the timeline summary and sprinkle in up to two highlights
    let mut summary = timeline.summary.clone();
    if summary.len() > 240 {
        summary.truncate(240);
        summary.push('…');
    }

    for highlight in timeline.highlights.iter().take(2) {
        if summary.len() + highlight.len() + 3 > 320 {
            break;
        }
        summary.push_str(" | ");
        summary.push_str(highlight);
    }

    Ok(summary)
}
