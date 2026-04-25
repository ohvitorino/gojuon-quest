pub(crate) const HIRAGANA_BASIC_46: [(&str, &str); 46] = [
    ("あ", "a"),
    ("い", "i"),
    ("う", "u"),
    ("え", "e"),
    ("お", "o"),
    ("か", "ka"),
    ("き", "ki"),
    ("く", "ku"),
    ("け", "ke"),
    ("こ", "ko"),
    ("さ", "sa"),
    ("し", "shi"),
    ("す", "su"),
    ("せ", "se"),
    ("そ", "so"),
    ("た", "ta"),
    ("ち", "chi"),
    ("つ", "tsu"),
    ("て", "te"),
    ("と", "to"),
    ("な", "na"),
    ("に", "ni"),
    ("ぬ", "nu"),
    ("ね", "ne"),
    ("の", "no"),
    ("は", "ha"),
    ("ひ", "hi"),
    ("ふ", "fu"),
    ("へ", "he"),
    ("ほ", "ho"),
    ("ま", "ma"),
    ("み", "mi"),
    ("む", "mu"),
    ("め", "me"),
    ("も", "mo"),
    ("や", "ya"),
    ("ゆ", "yu"),
    ("よ", "yo"),
    ("ら", "ra"),
    ("り", "ri"),
    ("る", "ru"),
    ("れ", "re"),
    ("ろ", "ro"),
    ("わ", "wa"),
    ("を", "wo"),
    ("ん", "n"),
];

pub(crate) const COLUMN_LABELS: [&str; 10] =
    ["Vowels", "K", "S", "T", "N", "H", "M", "Y", "R", "W"];

pub(crate) const COLUMN_INDEX_GROUPS: [&[usize]; 10] = [
    &[0, 1, 2, 3, 4],
    &[5, 6, 7, 8, 9],
    &[10, 11, 12, 13, 14],
    &[15, 16, 17, 18, 19],
    &[20, 21, 22, 23, 24],
    &[25, 26, 27, 28, 29],
    &[30, 31, 32, 33, 34],
    &[35, 36, 37],
    &[38, 39, 40, 41, 42],
    &[43, 44, 45],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hiragana_has_46_entries() {
        assert_eq!(HIRAGANA_BASIC_46.len(), 46);
    }

    #[test]
    fn column_labels_has_10_entries() {
        assert_eq!(COLUMN_LABELS.len(), 10);
    }

    #[test]
    fn column_index_groups_has_10_entries() {
        assert_eq!(COLUMN_INDEX_GROUPS.len(), 10);
    }

    #[test]
    fn column_index_groups_cover_all_46_indices_exactly_once() {
        let mut all_indices: Vec<usize> = COLUMN_INDEX_GROUPS
            .iter()
            .flat_map(|g| g.iter().copied())
            .collect();
        all_indices.sort_unstable();
        assert_eq!(all_indices, (0..46).collect::<Vec<_>>());
    }

    #[test]
    fn all_group_indices_within_bounds() {
        for group in COLUMN_INDEX_GROUPS.iter() {
            for &idx in *group {
                assert!(idx < HIRAGANA_BASIC_46.len());
            }
        }
    }

    #[test]
    fn irregular_romaji_mappings_are_correct() {
        assert_eq!(HIRAGANA_BASIC_46[11], ("し", "shi"));
        assert_eq!(HIRAGANA_BASIC_46[16], ("ち", "chi"));
        assert_eq!(HIRAGANA_BASIC_46[17], ("つ", "tsu"));
        assert_eq!(HIRAGANA_BASIC_46[27], ("ふ", "fu"));
        assert_eq!(HIRAGANA_BASIC_46[45], ("ん", "n"));
    }
}
