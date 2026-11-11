use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub struct FuzzySearch {
    matcher: SkimMatcherV2,
}

impl FuzzySearch {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn match_item(
        &self,
        item: &crate::model::MenuItem,
        query: &str,
    ) -> Option<(i64, Vec<usize>)> {
        // Match against both label and path
        let haystack = format!("{} {}", item.label, item.path);
        self.matcher.fuzzy_indices(&haystack, query)
    }

    pub fn filter<'a>(
        &self,
        items: &'a [crate::model::MenuItem],
        query: &str,
    ) -> Vec<(&'a crate::model::MenuItem, i64)> {
        let mut results: Vec<_> = items
            .iter()
            .filter_map(|item| self.match_item(item, query).map(|(score, _)| (item, score)))
            .collect();

        // Sort by score (higher is better)
        results.sort_by_key(|result| std::cmp::Reverse(result.1));
        results
    }
}

impl Default for FuzzySearch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        let search = FuzzySearch::new();
        let item = crate::model::MenuItem {
            id: "test".to_string(),
            label: "Background".to_string(),
            icon: "󰋩".to_string(),
            path: "Style › Background".to_string(),
            action_type: crate::model::ActionType::Submenu,
            script_path: None,
            is_submenu: true,
            is_active: false,
            level: "style".to_string(),
        };

        assert!(search.match_item(&item, "back").is_some());
        assert!(search.match_item(&item, "ground").is_some());
        assert!(search.match_item(&item, "xyz").is_none());
    }
}
