//! The advertised initialize result is the named smoke path.

#[cfg(test)]
mod tests
{
    use gandr_surface_lsp::TOKEN_TYPES;
    use gandr_surface_lsp::advertised_capabilities;

    #[test]
    fn advertised_capabilities_name_the_token_legend()
    {
        let caps = advertised_capabilities();
        let types = caps
            .pointer("/capabilities/semanticTokensProvider/legend/tokenTypes")
            .and_then(serde_json::Value::as_array)
            .expect("the initialize result carries the token legend");
        // The legend is a wire contract: a client indexes into it by integer,
        // so its ORDER is the contract and not merely its content. Stating it
        // literally here is deliberate — comparing the advertised legend
        // against `TOKEN_TYPES` would compare the implementation with itself
        // and could not fail. Changing this list is changing what every
        // already-configured client renders, so it should cost a red test.
        let expected: [&str; 14] = [
            "keyword",
            "operator",
            "function",
            "variable",
            "parameter",
            "property",
            "enumMember",
            "type",
            "typeParameter",
            "number",
            "string",
            "comment",
            "macro",
            "label",
        ];
        let advertised: Vec<&str> = types
            .iter()
            .map(|value| value.as_str().unwrap_or("<not a string>"))
            .collect();
        assert_eq!(expected.to_vec(), advertised);
        // The crate's exported legend is what the server advertises.
        assert_eq!(TOKEN_TYPES.len(), types.len());
        assert_eq!(
            Some(true),
            caps.pointer("/capabilities/hoverProvider")
                .and_then(serde_json::Value::as_bool)
        );
        assert!(
            caps.pointer("/capabilities/completionProvider").is_some(),
            "completion is part of the ordinary set"
        );
        // Both semantic-token request shapes are served. A client that reads
        // `range: false` never sends `semanticTokens/range` at all, so this is
        // the only place the range face is observable before a session starts.
        assert_eq!(
            Some(true),
            caps.pointer("/capabilities/semanticTokensProvider/full")
                .and_then(serde_json::Value::as_bool)
        );
        assert_eq!(
            Some(true),
            caps.pointer("/capabilities/semanticTokensProvider/range")
                .and_then(serde_json::Value::as_bool)
        );
    }
}
