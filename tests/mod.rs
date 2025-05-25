use view_type::views;

fn valid_ratio(ratio: Option<f32>) -> Option<f32> {
    ratio.filter(|&r| r >= 0.0 && r <= 1.0)
}


#[views(
    fragment all {
        offset,
        limit,
    }
    fragment keyword {
        Some(query),
        words_limit
    }
    fragment semantic {
        Some(vector)
    }
    struct KeywordSearch {
        ..all,
        ..keyword,
    }
    struct SemanticSearch<'a> {
        ..all,
        ..semantic,
    }
    struct HybridSearch<'a> {
        ..all,
        ..keyword,
        ..semantic,
        Some(ratio) = validate_ratio(ratio)
    }
)]
pub struct Search<'a> {
    query: Option<String>,
    offset: usize,
    limit: usize,
    words_limit: Option<usize>,
    vector: Option<&'a Vec<u8>>,
    ratio: Option<f32>,
}