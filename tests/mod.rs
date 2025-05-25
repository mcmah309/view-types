// // Usage example for the views macro



// // Your original struct with view annotations
// #[views(KeywordSearch, SemanticSearch, HybridSearch)]
// pub struct Search<'a> {
//     #[KeywordSearch(unwrap)]
//     query: Option<String>,
    
//     #[all]
//     filter: Option<Filter<'a>>,
    
//     #[all]
//     offset: usize,
    
//     #[all]
//     limit: usize,
    
//     #[all]
//     sort_criteria: Option<Vec<AscDesc>>,
    
//     #[all]
//     distinct: Option<String>,
    
//     #[KeywordSearch]
//     #[HybridSearch]
//     searchable_attributes: Option<&'a [String]>,
    
//     #[KeywordSearch]
//     #[HybridSearch]
//     geo_param: GeoSortParameter,
    
//     #[KeywordSearch]
//     #[HybridSearch]
//     terms_matching_strategy: TermsMatchingStrategy,
    
//     #[all]
//     scoring_strategy: ScoringStrategy,
    
//     #[KeywordSearch]
//     #[HybridSearch]
//     words_limit: usize,
    
//     #[all]
//     exhaustive_number_hits: bool,
    
//     #[all]
//     rtxn: &'a heed::RoTxn<'a>,
    
//     #[all]
//     index: &'a Index,
    
//     #[SemanticSearch]
//     semantic: Option<SemanticSearch>,
    
//     #[all]
//     time_budget: TimeBudget,
    
//     #[all]
//     ranking_score_threshold: Option<f64>,
    
//     #[KeywordSearch]
//     #[HybridSearch]
//     locales: Option<Vec<Language>>,
    
//     #[HybridSearch]
//     semantic_ratio: Option<f32>,
// }

// // Dummy types for the example
// pub struct Filter<'a>(&'a str);
// pub struct AscDesc;
// pub struct GeoSortParameter;
// pub struct TermsMatchingStrategy;
// pub struct ScoringStrategy;
// pub struct Index;
// pub struct SemanticSearch;
// pub struct TimeBudget;
// pub struct Language;

// fn main() {
//     // Create a Search instance
//     let search = Search {
//         query: Some("hello world".to_string()),
//         filter: None,
//         offset: 0,
//         limit: 10,
//         sort_criteria: None,
//         distinct: None,
//         searchable_attributes: None,
//         geo_param: GeoSortParameter,
//         terms_matching_strategy: TermsMatchingStrategy,
//         scoring_strategy: ScoringStrategy,
//         words_limit: 100,
//         exhaustive_number_hits: false,
//         rtxn: unsafe { std::mem::zeroed() }, // Obviously don't do this in real code
//         index: unsafe { std::mem::zeroed() },
//         semantic: Some(SemanticSearch),
//         time_budget: TimeBudget,
//         ranking_score_threshold: Some(0.5),
//         locales: None,
//         semantic_ratio: Some(0.7),
//     };

//     // Convert to different view types
//     if let Some(keyword_search) = search.clone().into_keyword_search() {
//         println!("Converted to keyword search with query: {}", keyword_search.query);
//         // Note: query is String, not Option<String> because of (unwrap)
//     }

//     if let Some(semantic_search) = search.clone().into_semantic_search() {
//         println!("Converted to semantic search");
//         // This has semantic field and all common fields
//     }

//     if let Some(hybrid_search) = search.clone().into_hybrid_search() {
//         println!("Converted to hybrid search with ratio: {:?}", hybrid_search.semantic_ratio);
//         // This has fields from both keyword and semantic, plus hybrid-specific fields
//     }

//     // Use reference views (zero-copy)
//     if let Some(keyword_ref) = search.as_keyword_search_ref() {
//         println!("Keyword search query reference: {}", keyword_ref.query);
//     }

//     if let Some(semantic_ref) = search.as_semantic_search_ref() {
//         println!("Semantic search reference created");
//     }

//     if let Some(hybrid_ref) = search.as_hybrid_search_ref() {
//         println!("Hybrid search reference created");
//     }
// }
