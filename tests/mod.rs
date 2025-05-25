// #[views(
//     frag all {
//         offset,
//         limit,
//     }
//     frag semantic {
//         Some(semantic) = valid_semantic_value(semantic),
//     }
//     frag keyword {
//         Some(query),
//         Some(searchable_attributes),
//         Some(terms_matching_strategy),
//         words_limit
//     }
//     struct KeywordSearch<'a> {
//         ..all,
//         ..keyword,
//     }
//     struct SemanticSearch {
//         ..all,
//         ..semantic,
//     }
//     struct HybridSearch<'a> { 
//         ..all,
//         ..semantic,
//         ..keyword,
//         semantic_ratio,
//     }
// )]
// pub struct Search<'a> {
//     query: String,
//     filter: Option<Filter<'a>>,
//     offset: usize,
//     limit: usize,
//     sort_criteria: Option<Vec<AscDesc>>,
//     distinct: Option<String>,
//     searchable_attributes: Option<&'a [String]>,
//     geo_param: new::GeoSortParameter,
//     terms_matching_strategy: TermsMatchingStrategy,
//     scoring_strategy: ScoringStrategy,
//     words_limit: usize,
//     exhaustive_number_hits: bool,
//     rtxn: &'a heed::RoTxn<'a>,
//     index: &'a Index,
//     semantic: Option<SemanticSearch>,
//     time_budget: TimeBudget,
//     ranking_score_threshold: Option<f64>,
//     locales: Option<Vec<Language>>,
//     semantic_ratio: Option<f32>
// }