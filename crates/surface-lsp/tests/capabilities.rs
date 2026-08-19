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
        assert_eq!(TOKEN_TYPES.len(), types.len());
        assert_eq!(
            Some("keyword"),
            types.first().and_then(serde_json::Value::as_str)
        );
        assert_eq!(
            Some(true),
            caps.pointer("/capabilities/hoverProvider")
                .and_then(serde_json::Value::as_bool)
        );
        assert!(
            caps.pointer("/capabilities/completionProvider").is_some(),
            "completion is part of the ordinary set"
        );
    }
}
