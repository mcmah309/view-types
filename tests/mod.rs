mod regular {
    use view_types::views;

    #[derive(Debug)]
    enum CannotInferType {
        Branch1(String),
        Branch2(usize),
    }

    fn validate_ratio(ratio: &f32) -> bool {
        *ratio >= 0.0 && *ratio <= 1.0
    }

    #[views(
        fragment all {
            offset,
            limit,
            CannotInferType::Branch1(cannot_infer_type: String),
            Ok(result1),
            Err(result2)
        }
        fragment keyword {
            Some(query),
            words_limit: Option<usize>
        }
        fragment semantic {
            Some(vector) if vector.len() == 768,
            mut_number
        }
        #[derive(Debug,Clone)]
        pub view KeywordSearch {
            ..all,
            ..keyword,
        }
        #[derive(Debug)]
        pub view SemanticSearch<'a> where 'a: 'a {
            ..all,
            ..semantic,
            semantic_only_ref
        }
        #[derive(Debug)]
        #[Ref(
            #[derive(Clone)]
        )]
        #[Mut(
            #[derive(Debug)]
        )]
        pub view HybridSearch<'a> {
            ..all,
            ..keyword,
            ..semantic,
            Some(ratio) if validate_ratio(ratio)
        }
    )]
    #[Variant(
        #[derive(Debug)]
    )]
    #[derive(Debug)]
    pub struct Search<'a> where 'a: 'a {
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
        result2: Result<usize, String>,
    }

    #[test]
    fn test() {
        let mut magic_number = 1;
        let vector = vec![0u8; 768];
        let semantic_only_ref = 100;
        let mut search = Search {
            query: Some("test".to_string()),
            offset: 0,
            limit: 10,
            words_limit: Some(5),
            vector: Some(&vector),
            ratio: Some(0.5),
            mut_number: &mut magic_number,
            semantic_only_ref: &semantic_only_ref,
            field_never_used: true,
            cannot_infer_type: CannotInferType::Branch1("branch1".to_owned()),
            result1: Ok(1),
            result2: Err("error".to_owned()),
        };

        let hybrid_ref: Option<HybridSearchRef<'_, '_>> = search.as_hybrid_search_ref();
        assert!(hybrid_ref.is_some());
        let hybrid = hybrid_ref.unwrap();
        assert_eq!(hybrid.offset, &0);
        assert_eq!(hybrid.limit, &10);
        assert_eq!(hybrid.query, &"test".to_string());
        assert_eq!(hybrid.words_limit, &Some(5));
        assert_eq!(hybrid.vector, &vector);
        assert_eq!(hybrid.ratio, &0.5);
        assert_eq!(hybrid.mut_number, &1);

        let hybrid_mut: Option<HybridSearchMut<'_, '_>> = search.as_hybrid_search_mut();
        assert!(hybrid_mut.is_some());
        let hybrid = hybrid_mut.unwrap();
        assert_eq!(hybrid.offset, &0);
        assert_eq!(hybrid.limit, &10);
        assert_eq!(hybrid.query, &"test".to_string());
        assert_eq!(hybrid.words_limit, &Some(5));
        assert_eq!(hybrid.vector, &vector);
        assert_eq!(hybrid.ratio, &0.5);
        assert_eq!(hybrid.mut_number, &1);
        *hybrid.mut_number += 1;
        assert_eq!(search.mut_number, &2);

        if let Some(ratio) = search.ratio.as_mut() {
            *ratio += 10.0;
        }

        assert!(search.as_hybrid_search_mut().is_none());
        assert!(search.as_hybrid_search_ref().is_none());

        let semantic_search = search.into_semantic_search();
        assert!(semantic_search.is_some());
        let mut semantic_search = semantic_search.unwrap();
        assert_eq!(semantic_search.offset, 0);
        assert_eq!(semantic_search.limit, 10);
        // etc

        let ref1 = semantic_search.as_ref();
        let ref2 = semantic_search.as_ref();
        assert_eq!(ref1.offset, ref2.offset);
        assert_eq!(ref1.limit, ref2.limit);

        let mutable = semantic_search.as_mut();
        assert_eq!(mutable.offset, &0);
        assert_eq!(mutable.limit, &10);
        *mutable.offset += 5;
        assert_eq!(mutable.offset, &5);
        *mutable.mut_number += 5;
        assert_eq!(mutable.mut_number, &7);
    }
}

mod builder {
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
            words_limit
        }
        fragment semantic {
            Some(vector) if vector.len() == 768,
            mut_number
        }
        view KeywordSearch {
            ..all,
            ..keyword,
        }
        view SemanticSearch<'a> {
            ..all,
            ..semantic,
        }
        view HybridSearch<'a> {
            ..all,
            ..keyword,
            ..semantic,
            Some(ratio) if validate_ratio(ratio)
        }
    )]
    #[derive(bon::Builder, Debug)]
    pub struct Search<'a> {
        query: Option<String>,
        #[builder(default = 1)]
        offset: usize,
        limit: usize,
        words_limit: Option<usize>,
        vector: Option<&'a Vec<u8>>,
        ratio: Option<f32>,
        mut_number: &'a mut usize,
        field_never_used: bool,
    }

    #[test]
    fn test() {
        let mut magic_number = 1;
        let vector = vec![0u8; 768];
        let mut search = Search::builder()
            .query("test".to_owned())
            .offset(0)
            .limit(10)
            .words_limit(5)
            .vector(&vector)
            .ratio(0.5)
            .mut_number(&mut magic_number)
            .field_never_used(true)
            .build();

        let hybrid_ref: Option<HybridSearchRef<'_, '_>> = search.as_hybrid_search_ref();
        assert!(hybrid_ref.is_some());
        let hybrid = hybrid_ref.unwrap();
        assert_eq!(hybrid.offset, &0);
        assert_eq!(hybrid.limit, &10);
        assert_eq!(hybrid.query, &"test".to_string());
        assert_eq!(hybrid.words_limit, &Some(5));
        assert_eq!(hybrid.vector, &vector);
        assert_eq!(hybrid.ratio, &0.5);
        assert_eq!(hybrid.mut_number, &1);

        let hybrid_mut: Option<HybridSearchMut<'_, '_>> = search.as_hybrid_search_mut();
        assert!(hybrid_mut.is_some());
        let hybrid = hybrid_mut.unwrap();
        assert_eq!(hybrid.offset, &0);
        assert_eq!(hybrid.limit, &10);
        assert_eq!(hybrid.query, &"test".to_string());
        assert_eq!(hybrid.words_limit, &Some(5));
        assert_eq!(hybrid.vector, &vector);
        assert_eq!(hybrid.ratio, &0.5);
        assert_eq!(hybrid.mut_number, &1);
        *hybrid.mut_number += 1;
        assert_eq!(search.mut_number, &2);

        if let Some(ratio) = search.ratio.as_mut() {
            *ratio += 10.0;
        }

        assert!(search.as_hybrid_search_mut().is_none());
        assert!(search.as_hybrid_search_ref().is_none());
    }
}
