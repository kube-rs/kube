/// Filters out empty strings.
pub(crate) fn nonempty(string: Option<String>) -> Option<String> {
    string.filter(|s| !s.is_empty())
}
