# View-types: A Concise Way To Model Data With View Projections

[<img alt="crates.io" src="https://img.shields.io/crates/v/view-types.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/view-types)

The `views` macro provides a declarative way to define type-safe projections from a single source-of-truth data structure declaration. These projections provide different ways of representing data with overlapping fields or needing runtime validation, and minimizes the necessary boilerplate. This can even be made more powerful when combined with the builder pattern ([example](https://github.com/mcmah309/view-types/blob/1137d4bb7a20d405d01a5a6c79ddb19c158a5c89/tests/mod.rs#L182) with [bon](https://crates.io/crates/bon)).

Jump straight to [examples](#examples) to see it in action.

Article: [Solving Rust Data Modeling with View-Types: A Macro-Driven Approach](https://mcmah309.github.io/posts/solving-data-modeling-in-rust-with-view-types/)

## Syntax

### Syntax Example

```rust
use view_types::views;

fn validate_ratio(ratio: &f32) -> bool {
    *ratio >= 0.0 && *ratio <= 1.0
}

enum EnumVariant {
    Branch1(String),
    Branch2(usize),
}

#[views(
    // A fragment is a set of fields to be included in view(s)
    frag all {
        // Declaring a field to be included
        offset,
        limit,
        // Enum pattern matching extraction with explicit type declaration
        EnumVariant::Branch1(cannot_infer_type: String),
        // Result pattern matching extraction
        Ok(result1),
    }
    
    frag keyword {
        // Option pattern matching extraction
        Some(query),
        // Explicit type declaration
        words_limit: Option<usize>
    }
    
    frag semantic {
        // Option pattern matching extraction with validation
        Some(vector) if vector.len() == 768,
        mut_number
    }
    
    // A view is a projection/subset of fields
    #[derive(Debug, Clone)]
    pub view KeywordSearch {
        // Expanding a fragment to include all fields in this view
        ..all,
        ..keyword,
    }
    
    #[derive(Debug)]
    pub view SemanticSearch<'a> where 'a: 'a {
        ..all,
        ..semantic,
        // Directly declaring a field (same as in a fragment)
        semantic_only_ref
    }
    
    #[derive(Debug)]
    pub view HybridSearch<'a> {
        ..all,
        ..keyword,
        ..semantic,
        Some(ratio) if validate_ratio(ratio)
    }
)]
pub struct Search<'a> {
    query: Option<String>,
    offset: usize,
    limit: usize,
    words_limit: Option<usize>,
    vector: Option<&'a Vec<u8>>,
    ratio: Option<f32>,
    mut_number: &'a mut usize,
    field_never_used: bool,
    semantic_only_ref: &'a usize,
    cannot_infer_type: EnumVariant,
    result1: Result<usize, String>,
}
```

See the macro expansion below to understand the generated code.
<details>

<summary>Expansion</summary>

```rust,ignore
// Recursive expansion of views macro
// ===================================

pub struct Search<'a> {
    query: Option<String>,
    offset: usize,
    limit: usize,
    words_limit: Option<usize>,
    vector: Option<&'a Vec<u8>>,
    ratio: Option<f32>,
    mut_number: &'a mut usize,
    field_never_used: bool,
    semantic_only_ref: &'a usize,
    cannot_infer_type: EnumVariant,
    result1: Result<usize, String>,
}
#[derive(Debug, Clone)]
pub struct KeywordSearch {
    offset: usize,
    limit: usize,
    cannot_infer_type: String,
    result1: usize,
    query: String,
    words_limit: Option<usize>,
}
pub struct KeywordSearchRef<'original> {
    offset: &'original usize,
    limit: &'original usize,
    cannot_infer_type: &'original String,
    result1: &'original usize,
    query: &'original String,
    words_limit: &'original Option<usize>,
}
pub struct KeywordSearchMut<'original> {
    offset: &'original mut usize,
    limit: &'original mut usize,
    cannot_infer_type: &'original mut String,
    result1: &'original mut usize,
    query: &'original mut String,
    words_limit: &'original mut Option<usize>,
}
impl<'original> KeywordSearch {
    pub fn as_ref(&'original self) -> KeywordSearchRef<'original> {
        KeywordSearchRef {
            offset: &self.offset,
            limit: &self.limit,
            cannot_infer_type: &self.cannot_infer_type,
            result1: &self.result1,
            query: &self.query,
            words_limit: &self.words_limit,
        }
    }
    pub fn as_mut(&'original mut self) -> KeywordSearchMut<'original> {
        KeywordSearchMut {
            offset: &mut self.offset,
            limit: &mut self.limit,
            cannot_infer_type: &mut self.cannot_infer_type,
            result1: &mut self.result1,
            query: &mut self.query,
            words_limit: &mut self.words_limit,
        }
    }
}
#[derive(Debug)]
pub struct SemanticSearch<'a>
where
    'a: 'a,
{
    offset: usize,
    limit: usize,
    cannot_infer_type: String,
    result1: usize,
    vector: &'a Vec<u8>,
    mut_number: &'a mut usize,
    semantic_only_ref: &'a usize,
}
pub struct SemanticSearchRef<'original, 'a>
where
    'a: 'a,
{
    offset: &'original usize,
    limit: &'original usize,
    cannot_infer_type: &'original String,
    result1: &'original usize,
    vector: &'a Vec<u8>,
    mut_number: &'original usize,
    semantic_only_ref: &'a usize,
}
pub struct SemanticSearchMut<'original, 'a>
where
    'a: 'a,
{
    offset: &'original mut usize,
    limit: &'original mut usize,
    cannot_infer_type: &'original mut String,
    result1: &'original mut usize,
    vector: &'a Vec<u8>,
    mut_number: &'original mut usize,
    semantic_only_ref: &'a usize,
}
impl<'original, 'a> SemanticSearch<'a>
where
    'a: 'a,
{
    pub fn as_ref(&'original self) -> SemanticSearchRef<'original, 'a> {
        SemanticSearchRef {
            offset: &self.offset,
            limit: &self.limit,
            cannot_infer_type: &self.cannot_infer_type,
            result1: &self.result1,
            vector: &self.vector,
            mut_number: &self.mut_number,
            semantic_only_ref: &self.semantic_only_ref,
        }
    }
    pub fn as_mut(&'original mut self) -> SemanticSearchMut<'original, 'a> {
        SemanticSearchMut {
            offset: &mut self.offset,
            limit: &mut self.limit,
            cannot_infer_type: &mut self.cannot_infer_type,
            result1: &mut self.result1,
            vector: &mut self.vector,
            mut_number: &mut self.mut_number,
            semantic_only_ref: &mut self.semantic_only_ref,
        }
    }
}
#[derive(Debug)]
pub struct HybridSearch<'a> {
    offset: usize,
    limit: usize,
    cannot_infer_type: String,
    result1: usize,
    query: String,
    words_limit: Option<usize>,
    vector: &'a Vec<u8>,
    mut_number: &'a mut usize,
    ratio: f32,
}
pub struct HybridSearchRef<'original, 'a> {
    offset: &'original usize,
    limit: &'original usize,
    cannot_infer_type: &'original String,
    result1: &'original usize,
    query: &'original String,
    words_limit: &'original Option<usize>,
    vector: &'a Vec<u8>,
    mut_number: &'original usize,
    ratio: &'original f32,
}
pub struct HybridSearchMut<'original, 'a> {
    offset: &'original mut usize,
    limit: &'original mut usize,
    cannot_infer_type: &'original mut String,
    result1: &'original mut usize,
    query: &'original mut String,
    words_limit: &'original mut Option<usize>,
    vector: &'a Vec<u8>,
    mut_number: &'original mut usize,
    ratio: &'original mut f32,
}
impl<'original, 'a> HybridSearch<'a> {
    pub fn as_ref(&'original self) -> HybridSearchRef<'original, 'a> {
        HybridSearchRef {
            offset: &self.offset,
            limit: &self.limit,
            cannot_infer_type: &self.cannot_infer_type,
            result1: &self.result1,
            query: &self.query,
            words_limit: &self.words_limit,
            vector: &self.vector,
            mut_number: &self.mut_number,
            ratio: &self.ratio,
        }
    }
    pub fn as_mut(&'original mut self) -> HybridSearchMut<'original, 'a> {
        HybridSearchMut {
            offset: &mut self.offset,
            limit: &mut self.limit,
            cannot_infer_type: &mut self.cannot_infer_type,
            result1: &mut self.result1,
            query: &mut self.query,
            words_limit: &mut self.words_limit,
            vector: &mut self.vector,
            mut_number: &mut self.mut_number,
            ratio: &mut self.ratio,
        }
    }
}
pub enum SearchVariant<'a> {
    KeywordSearch(KeywordSearch),
    SemanticSearch(SemanticSearch<'a>),
    HybridSearch(HybridSearch<'a>),
}
impl<'a> SearchVariant<'a> {
    pub fn offset(&self) -> &usize {
        match self {
            SearchVariant::KeywordSearch(view) => &view.offset,
            SearchVariant::SemanticSearch(view) => &view.offset,
            SearchVariant::HybridSearch(view) => &view.offset,
        }
    }
    pub fn vector(&self) -> Option<&Vec<u8>> {
        match self {
            SearchVariant::SemanticSearch(view) => Some(&view.vector),
            SearchVariant::HybridSearch(view) => Some(&view.vector),
            _ => None,
        }
    }
    pub fn words_limit(&self) -> Option<&usize> {
        match self {
            SearchVariant::KeywordSearch(view) => view.words_limit.as_ref(),
            SearchVariant::HybridSearch(view) => view.words_limit.as_ref(),
            _ => None,
        }
    }
    pub fn semantic_only_ref(&self) -> Option<&usize> {
        match self {
            SearchVariant::SemanticSearch(view) => Some(&view.semantic_only_ref),
            _ => None,
        }
    }
    pub fn cannot_infer_type(&self) -> &String {
        match self {
            SearchVariant::KeywordSearch(view) => &view.cannot_infer_type,
            SearchVariant::SemanticSearch(view) => &view.cannot_infer_type,
            SearchVariant::HybridSearch(view) => &view.cannot_infer_type,
        }
    }
    pub fn query(&self) -> Option<&String> {
        match self {
            SearchVariant::KeywordSearch(view) => Some(&view.query),
            SearchVariant::HybridSearch(view) => Some(&view.query),
            _ => None,
        }
    }
    pub fn ratio(&self) -> Option<&f32> {
        match self {
            SearchVariant::HybridSearch(view) => Some(&view.ratio),
            _ => None,
        }
    }
    pub fn result1(&self) -> &usize {
        match self {
            SearchVariant::KeywordSearch(view) => &view.result1,
            SearchVariant::SemanticSearch(view) => &view.result1,
            SearchVariant::HybridSearch(view) => &view.result1,
        }
    }
    pub fn limit(&self) -> &usize {
        match self {
            SearchVariant::KeywordSearch(view) => &view.limit,
            SearchVariant::SemanticSearch(view) => &view.limit,
            SearchVariant::HybridSearch(view) => &view.limit,
        }
    }
    pub fn mut_number(&self) -> Option<&usize> {
        match self {
            SearchVariant::SemanticSearch(view) => Some(&view.mut_number),
            SearchVariant::HybridSearch(view) => Some(&view.mut_number),
            _ => None,
        }
    }
}
impl<'original, 'a> Search<'a> {
    pub fn into_keyword_search(self) -> Option<KeywordSearch> {
        Some(KeywordSearch {
            offset: self.offset,
            limit: self.limit,
            cannot_infer_type: if let EnumVariant::Branch1(cannot_infer_type) =
                self.cannot_infer_type
            {
                cannot_infer_type
            } else {
                return None;
            },
            result1: if let Ok(result1) = self.result1 {
                result1
            } else {
                return None;
            },
            query: if let Some(query) = self.query {
                query
            } else {
                return None;
            },
            words_limit: self.words_limit,
        })
    }
    pub fn as_keyword_search(&'original self) -> Option<KeywordSearchRef<'original>> {
        Some(KeywordSearchRef {
            offset: &self.offset,
            limit: &self.limit,
            cannot_infer_type: if let EnumVariant::Branch1(cannot_infer_type) =
                &self.cannot_infer_type
            {
                cannot_infer_type
            } else {
                return None;
            },
            result1: if let Ok(result1) = &self.result1 {
                result1
            } else {
                return None;
            },
            query: if let Some(query) = &self.query {
                query
            } else {
                return None;
            },
            words_limit: &self.words_limit,
        })
    }
    pub fn as_keyword_search_mut(&'original mut self) -> Option<KeywordSearchMut<'original>> {
        Some(KeywordSearchMut {
            offset: {
                let offset = &mut self.offset;
                offset
            },
            limit: {
                let limit = &mut self.limit;
                limit
            },
            cannot_infer_type: if let EnumVariant::Branch1(cannot_infer_type) =
                &mut self.cannot_infer_type
            {
                cannot_infer_type
            } else {
                return None;
            },
            result1: if let Ok(result1) = &mut self.result1 {
                result1
            } else {
                return None;
            },
            query: if let Some(query) = &mut self.query {
                query
            } else {
                return None;
            },
            words_limit: {
                let words_limit = &mut self.words_limit;
                words_limit
            },
        })
    }
    pub fn into_semantic_search(self) -> Option<SemanticSearch<'a>> {
        Some(SemanticSearch {
            offset: self.offset,
            limit: self.limit,
            cannot_infer_type: if let EnumVariant::Branch1(cannot_infer_type) =
                self.cannot_infer_type
            {
                cannot_infer_type
            } else {
                return None;
            },
            result1: if let Ok(result1) = self.result1 {
                result1
            } else {
                return None;
            },
            vector: if let Some(vector) = self.vector {
                {
                    let vector = &vector;
                    if !(vector.len() == 768) {
                        return None;
                    }
                }
                vector
            } else {
                return None;
            },
            mut_number: self.mut_number,
            semantic_only_ref: self.semantic_only_ref,
        })
    }
    pub fn as_semantic_search(&'original self) -> Option<SemanticSearchRef<'original, 'a>> {
        Some(SemanticSearchRef {
            offset: &self.offset,
            limit: &self.limit,
            cannot_infer_type: if let EnumVariant::Branch1(cannot_infer_type) =
                &self.cannot_infer_type
            {
                cannot_infer_type
            } else {
                return None;
            },
            result1: if let Ok(result1) = &self.result1 {
                result1
            } else {
                return None;
            },
            vector: if let Some(vector) = &self.vector {
                if !(vector.len() == 768) {
                    return None;
                }
                vector
            } else {
                return None;
            },
            mut_number: &self.mut_number,
            semantic_only_ref: &self.semantic_only_ref,
        })
    }
    pub fn as_semantic_search_mut(&'original mut self) -> Option<SemanticSearchMut<'original, 'a>> {
        Some(SemanticSearchMut {
            offset: {
                let offset = &mut self.offset;
                offset
            },
            limit: {
                let limit = &mut self.limit;
                limit
            },
            cannot_infer_type: if let EnumVariant::Branch1(cannot_infer_type) =
                &mut self.cannot_infer_type
            {
                cannot_infer_type
            } else {
                return None;
            },
            result1: if let Ok(result1) = &mut self.result1 {
                result1
            } else {
                return None;
            },
            vector: if let Some(vector) = &mut self.vector {
                {
                    let vector = &*vector;
                    if !(vector.len() == 768) {
                        return None;
                    }
                }
                vector
            } else {
                return None;
            },
            mut_number: {
                let mut_number = &mut self.mut_number;
                &mut *mut_number
            },
            semantic_only_ref: {
                let semantic_only_ref = &mut self.semantic_only_ref;
                semantic_only_ref
            },
        })
    }
    pub fn into_hybrid_search(self) -> Option<HybridSearch<'a>> {
        Some(HybridSearch {
            offset: self.offset,
            limit: self.limit,
            cannot_infer_type: if let EnumVariant::Branch1(cannot_infer_type) =
                self.cannot_infer_type
            {
                cannot_infer_type
            } else {
                return None;
            },
            result1: if let Ok(result1) = self.result1 {
                result1
            } else {
                return None;
            },
            query: if let Some(query) = self.query {
                query
            } else {
                return None;
            },
            words_limit: self.words_limit,
            vector: if let Some(vector) = self.vector {
                {
                    let vector = &vector;
                    if !(vector.len() == 768) {
                        return None;
                    }
                }
                vector
            } else {
                return None;
            },
            mut_number: self.mut_number,
            ratio: if let Some(ratio) = self.ratio {
                {
                    let ratio = &ratio;
                    if !(validate_ratio(ratio)) {
                        return None;
                    }
                }
                ratio
            } else {
                return None;
            },
        })
    }
    pub fn as_hybrid_search(&'original self) -> Option<HybridSearchRef<'original, 'a>> {
        Some(HybridSearchRef {
            offset: &self.offset,
            limit: &self.limit,
            cannot_infer_type: if let EnumVariant::Branch1(cannot_infer_type) =
                &self.cannot_infer_type
            {
                cannot_infer_type
            } else {
                return None;
            },
            result1: if let Ok(result1) = &self.result1 {
                result1
            } else {
                return None;
            },
            query: if let Some(query) = &self.query {
                query
            } else {
                return None;
            },
            words_limit: &self.words_limit,
            vector: if let Some(vector) = &self.vector {
                if !(vector.len() == 768) {
                    return None;
                }
                vector
            } else {
                return None;
            },
            mut_number: &self.mut_number,
            ratio: if let Some(ratio) = &self.ratio {
                if !(validate_ratio(ratio)) {
                    return None;
                }
                ratio
            } else {
                return None;
            },
        })
    }
    pub fn as_hybrid_search_mut(&'original mut self) -> Option<HybridSearchMut<'original, 'a>> {
        Some(HybridSearchMut {
            offset: {
                let offset = &mut self.offset;
                offset
            },
            limit: {
                let limit = &mut self.limit;
                limit
            },
            cannot_infer_type: if let EnumVariant::Branch1(cannot_infer_type) =
                &mut self.cannot_infer_type
            {
                cannot_infer_type
            } else {
                return None;
            },
            result1: if let Ok(result1) = &mut self.result1 {
                result1
            } else {
                return None;
            },
            query: if let Some(query) = &mut self.query {
                query
            } else {
                return None;
            },
            words_limit: {
                let words_limit = &mut self.words_limit;
                words_limit
            },
            vector: if let Some(vector) = &mut self.vector {
                {
                    let vector = &*vector;
                    if !(vector.len() == 768) {
                        return None;
                    }
                }
                vector
            } else {
                return None;
            },
            mut_number: {
                let mut_number = &mut self.mut_number;
                &mut *mut_number
            },
            ratio: if let Some(ratio) = &mut self.ratio {
                {
                    let ratio = &*ratio;
                    if !(validate_ratio(ratio)) {
                        return None;
                    }
                }
                ratio
            } else {
                return None;
            },
        })
    }
}
```

</details>

### Fragment-Based Grouping

Fragments allow you to group related field extractions and reuse them across multiple views:

```rust,ignore
frag all {
    offset,                                           // Simple field extraction
    limit,
    EnumVariant::Branch1(cannot_infer_type: String), // Enum pattern matching
    Ok(result1),                                      // Result unwrapping
    Err(result2)
}
```

The macro supports conditional field extraction with custom validation:

```rust,ignore
frag semantic {
    Some(vector) if vector.len() == 768,  // Conditional extraction
    mut_number
}
```

### Views

Views are projections of the annotated structs data. They contain fragments and fields to be included in the projection.

```rust
// Annotations for the generated *Ref struct
#[Ref(
    #[derive(Clone)]
)]
// Annotations for the generated *Mut struct
#[Mut(
    #[derive(Debug)]
)]
#[derive(Debug)]
pub view HybridSearch<'a> {
    // fragment expansion
    ..all,
    ..keyword,
    ..semantic,
    // direct field inclusion with pattern matching and validation (same syntax as in a fragment)
    Some(ratio) if validate_ratio(ratio)
}
```
### Configuration
#### Variant
In addition to the structs generated for each view (each view has a owned, ref, and mut struct). There is also a generated enum variant of the views. e.g.
```rust
pub enum SearchVariant<'a> {
    KeywordSearch(KeywordSearch),
    SemanticSearch(SemanticSearch<'a>),
    HybridSearch(HybridSearch<'a>),
}
```
Annotations for this type can be applied with the `Variant` annotation directly on the original struct.
```rust
#[Variant(
    #[derive(Debug)]
)]
```

## Examples

### Example Using Monolith

```rust
use view_types::views;

fn validate_table_name(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.is_empty()
}

fn validate_limit(limit: &u32) -> bool {
    *limit > 0 && *limit <= 10000
}

#[derive(Debug, Clone)]
pub struct JoinClause {
    pub table: String,
    pub condition: String,
}

#[views(
    frag base {
        Some(table) if validate_table_name(table),
        columns,
    }

    #[derive(Debug, Clone)]
    pub view SelectQuery {
        ..base
    }
    
    #[derive(Debug, Clone)]  
    pub view PaginatedQuery {
        ..base,
        Some(limit) if validate_limit(limit),
        Some(offset),
    }
    
    #[derive(Debug, Clone)]
    pub view JoinQuery {
        ..base,
        Some(join_clauses) if !join_clauses.is_empty(),
    }
)]
#[derive(Debug)]
pub struct QueryBuilder {
    table: Option<String>,
    columns: Vec<String>,
    where_clause: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
    join_clauses: Option<Vec<JoinClause>>,
}


fn configure_select_query(query: &SelectQueryRef, sql: &mut String) {
    let cols = if query.columns.is_empty() { "*" } else { &query.columns.join(", ") };
    sql.push_str(&format!("SELECT {} FROM {} ", cols, query.table));
}

fn configure_join_query(query: &JoinQueryRef, sql: &mut String) {
    for join in query.join_clauses {
        sql.push_str(&format!(" JOIN {} ON {}", join.table, join.condition));
    }
}

fn configure_paginated_query(query: &PaginatedQueryRef, sql: &mut String) {
    sql.push_str(&format!(" LIMIT {} OFFSET {}", query.limit, query.offset));
}

fn main() {
    // Assume unknown query configuration (could come from API request, config file, etc.)
    let query_builder = QueryBuilder {
        table: Some("users".to_string()),
        columns: vec!["id".to_string(), "name".to_string(), "email".to_string()],
        where_clause: Some("active = true".to_string()),
        limit: Some(50),
        offset: Some(0),
        join_clauses: Some(vec![JoinClause {
            table: "profiles".to_string(),
            condition: "users.id = profiles.user_id".to_string(),
        }]),
    };
    
    let mut sql = String::new();
    if let Some(query) = query_builder.as_select_query() {
        configure_select_query(&query, &mut sql);
    }
    else {
        panic!("Not valid query");
    }
    if let Some(query) = query_builder.as_join_query() {
        configure_join_query(&query, &mut sql);
    }
    if let Some(query) = query_builder.as_paginated_query() {
        configure_paginated_query(&query, &mut sql);
    }
    if let Some(where_clause) = query_builder.where_clause {
        sql.push_str(&format!(" WHERE {}", where_clause));
    }

    println!("Generated SQL Query: {}", sql);
}
```

### Example Using Generated Variant Enum

```rust
use view_types::views;

// Debug only validation
#[inline]
fn validate_health(health: &f32) -> bool {
    #[cfg(debug_assertions)]
    { *health >= 0.0 && *health <= 100.0 }
    #[cfg(not(debug_assertions))]
    { true }
}

#[derive(Debug, Clone)]
pub enum Team { Blue, Red, Neutral }

#[derive(Debug, Clone)]
pub enum WeaponType { Sword, Bow, Staff }

#[views(
    frag positioned {
        entity_id,
        position_x,
        position_y,
    }
    
    frag living {
        Some(health) if validate_health(health),
        max_health,
        team,
    }
    
    #[derive(Debug, Clone)]
    pub view Player {
        ..positioned,
        ..living,
        player_name,
        level,
        weapon,
    }
    
    #[derive(Debug, Clone)]
    pub view Npc {
        ..positioned,
        ..living,
        npc_type,
        ai_state,
    }
    
    #[derive(Debug, Clone)]
    pub view Projectile {
        ..positioned,
        velocity_x,
        velocity_y,
        team,
        damage: u32,
    }
)]
pub struct GameEntity {
    entity_id: u64,
    position_x: f32,
    position_y: f32,
    health: Option<f32>,
    max_health: f32,
    team: Team,
    damage: u32,
    player_name: String,
    level: u32,
    weapon: WeaponType,
    npc_type: String,
    ai_state: String,
    velocity_x: f32,
    velocity_y: f32,
}

fn main() {
    // Simulate game entities
    let entities = vec![
        GameEntityVariant::Player(Player {
            entity_id: 1,
            position_x: 100.0,
            position_y: 200.0,
            health: 85.0,
            max_health: 100.0,
            team: Team::Blue,
            player_name: "Alice".to_string(),
            level: 12,
            weapon: WeaponType::Sword,
        }),
        
        GameEntityVariant::Npc(Npc {
            entity_id: 2,
            position_x: 300.0,
            position_y: 150.0,
            health: 60.0,
            max_health: 80.0,
            team: Team::Neutral,
            npc_type: "Merchant".to_string(),
            ai_state: "Idle".to_string(),
        }),
        
        GameEntityVariant::Projectile(Projectile {
            entity_id: 3,
            position_x: 120.0,
            position_y: 210.0,
            velocity_x: 200.0,
            velocity_y: -50.0,
            team: Team::Blue,
            damage: 25,
        }),
    ];
    
    // Use generated getters directly - no pattern matching required!
    for entity in &entities {
        // These are common for all so it is automatically generated without an option
        println!("Entity {} at ({:.0}, {:.0})", entity.entity_id(), entity.position_x(), entity.position_y());
        let near_center = entities.iter()
            .filter(|e| {
                let dx = e.position_x() - 200.0;
                let dy = e.position_y() - 175.0;
                dx * dx + dy * dy <= 100.0 * 100.0
            })
        .count();
        println!("\nEntities near center: {}", near_center);
        
        // Access fields that exist only in some variants (uses options)
        if let Some(health) = entity.health() {
            println!("  Health: {:.0}/{:.0}", health, entity.max_health().unwrap());
        }
        if let Some(player_name) = entity.player_name() {
            println!("  Player: {}", player_name);
        }
        if let Some(npc_type) = entity.npc_type() {
            println!("  NPC: {}", npc_type);
        }
    }
    
    // Pattern matching for type-specific behavior
    for entity in &entities {
        match entity {
            GameEntityVariant::Player(player) => {
                println!("Player {} with {:?}", player.player_name, player.weapon);
            },
            GameEntityVariant::Projectile(proj) => {
                println!("Projectile: damage {}, velocity ({:.0}, {:.0})", 
                    proj.damage, proj.velocity_x, proj.velocity_y);
            },
            _ => {
                println!("Other entity type");
            }
        }
    }
}
```