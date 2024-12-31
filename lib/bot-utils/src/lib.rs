pub fn collect_somes<'a, I>(s: I) -> Option<Vec<String>>
where I: IntoIterator<Item = &'a Option<&'a String>>
{
    let string_list = s.into_iter()
        .filter(|f| f.is_some())
        .map(|s| s.unwrap().to_string())
        .collect::<Vec<String>>();

    if string_list.is_empty() {
        None
    } else {
        Some(string_list)
    }
}
