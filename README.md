# View-types: A Concise Approach to Complex Data Modeling in Rust

The `views` macro provides a declarative way to define type-safe projections from a single source-of-truth data structure declaration. These projections provide different ways of modeling data and minimizes the necessary boilerplate.

## Basic Syntax And Usage

```rust
use view_types::views;

fn validate_ratio(ratio: &f32) -> bool {
    *ratio >= 0.0 && *ratio <= 1.0
}

enum CannotInferType {
    Branch1(String),
    Branch2(usize),
}

#[views(
    // A fragment is a set of fields to be included in view(s)
    fragment all {
        // Declaring a field to be included
        offset,
        limit,
        // branch extraction with pattern matching
        CannotInferType::Branch1(cannot_infer_type: String),
        // Result unwrapping
        Ok(result1),
    }
    
    fragment keyword {
        // Option unwrapping
        Some(query),
        // Explicit type declaration
        words_limit: Option<usize>
    }
    
    fragment semantic {
        // Option unwrapping with validation
        Some(vector) if vector.len() == 768,
        mut_number
    }
    
    // A view is a projection/subset of fields
    #[derive(Debug, Clone)]
    pub view KeywordSearch {
        // Expanding a fragment to include all fields
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
    cannot_infer_type: CannotInferType,
    result1: Result<usize, String>,
}
```
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
    cannot_infer_type: CannotInferType,
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
impl<'original, 'a> Search<'a> {
    pub fn into_keyword_search(self) -> Option<KeywordSearch> {
        Some(KeywordSearch {
            offset: self.offset,
            limit: self.limit,
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) =
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
    pub fn as_keyword_search_ref(&'original self) -> Option<KeywordSearchRef<'original>> {
        Some(KeywordSearchRef {
            offset: &self.offset,
            limit: &self.limit,
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) =
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
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) =
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
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) =
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
    pub fn as_semantic_search_ref(&'original self) -> Option<SemanticSearchRef<'original, 'a>> {
        Some(SemanticSearchRef {
            offset: &self.offset,
            limit: &self.limit,
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) =
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
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) =
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
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) =
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
    pub fn as_hybrid_search_ref(&'original self) -> Option<HybridSearchRef<'original, 'a>> {
        Some(HybridSearchRef {
            offset: &self.offset,
            limit: &self.limit,
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) =
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
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) =
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

## Declaration

### Fragment-Based Composition

Fragments allow you to group related field extractions and reuse them across multiple views:

```rust,ignore
fragment all {
    offset,                                           // Simple field extraction
    limit,
    CannotInferType::Branch1(cannot_infer_type: String), // Enum pattern matching
    Ok(result1),                                      // Result unwrapping
    Err(result2)
}
```

### Runtime Validation with Compile-Time Safety

The macro supports conditional field extraction with custom validation:

```rust,ignore
fragment semantic {
    Some(vector) if vector.len() == 768,  // Conditional extraction
    mut_number
}
```

## Generation

### Automatic Type Generation

The macro automatically generates a corresponding type and reference types with proper lifetime management for each view:

```rust,ignore
// From this declaration:
pub view KeywordSearch {
    ..all,
    ..keyword,
}
...
```
```rust,ignore
// Generated automatically
pub struct KeywordSearch {
    offset: usize,
    limit: usize,
    query: String,
    // ... other fields
}

pub struct KeywordSearchRef<'original> {
    offset: &'original usize,
    limit: &'original usize,
    query: &'original String,
    // ... other fields
}

pub struct KeywordSearchMut<'original> {
    offset: &'original mut usize,
    limit: &'original mut usize,
    query: &'original mut String,
    // ... other fields
}
```

### Access Patterns

For each view, the macro generates three access methods e.g.:

- **Owned conversion**: `into_keyword_search()` - Returns ``Option<KeywordSearch>`
- **Immutable borrowing**: `as_keyword_search_ref()` - Returns `Option<KeywordSearchRef<'...>>`
- **Mutable borrowing**: `as_keyword_search_mut()` - Returns `Option<KeywordSearchMut<'...>>`

e.g.

```rust,ignore
impl Search<'_> {
    pub fn into_keyword_search(self) -> Option<KeywordSearch> {
        Some(KeywordSearch {
            offset: self.offset,
            limit: self.limit,
            cannot_infer_type: if let CannotInferType::Branch1(cannot_infer_type) = 
                self.cannot_infer_type {
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
}
```

## Usage Examples

### Basic Conversion

```rust
use view_types::views;

fn validate_ratio(ratio: &f32) -> bool {
    *ratio >= 0.0 && *ratio <= 1.0
}

#[views(
    fragment all {
        offset,
        limit,
    }
    
    fragment keyword {
        Some(query),
        words_limit: Option<usize>
    }
    
    fragment semantic {
        Some(vector) if vector.len() == 768,
        mut_number
    }

    pub view KeywordSearch {
        ..all,
        ..keyword,
    }
    pub view SemanticSearch<'a> {
        ..all,
        ..semantic,
        semantic_only_ref
    }
    
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
    semantic_only_ref: &'a usize,
}

fn main() {
    let mut magic_number = 1;
    let vector = vec![0u8; 768];
    let semantic_only_ref = 100;

    let mut search = Search {
        query: Some("rust search".to_string()),
        offset: 0,
        limit: 10,
        words_limit: Some(5),
        vector: Some(&vector),
        ratio: Some(0.5),
        mut_number: &mut magic_number,
        semantic_only_ref: &semantic_only_ref,
    };

    // Try to convert to different view types
    if let Some(keyword) = search.as_keyword_search_ref() {
        println!("Query: {}", keyword.query);
        println!("Offset: {}", keyword.offset);
    }

    if let Some(mut hybrid) = search.as_hybrid_search_mut() {
        *hybrid.mut_number += 1;  // Modify through the view
        println!("Ratio: {}", hybrid.ratio);
    }

    let semantic_search = search.into_semantic_search().unwrap();
    println!("Vector length: {}", semantic_search.vector.len());
}
```