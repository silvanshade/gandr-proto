abstract GandrDocs = {
  flags startcat = Component ;
  cat
    Component ; Section ; Block ; Inline ;
    Status ; Anchor ;
    Term ; CiteKey ;
    Production ; MathRow ; Cell ; Row ; Item ;
    [Section] ; [Block] ; [Inline] ; [Production] ; [MathRow] ; [Cell] ; [Row] ; [Item] ; [CiteKey] ; [Anchor] ;
  fun
    -- component and sections
    MkComponent : Anchor -> String -> Status -> [Anchor] -> [Anchor] -> [Section] -> [CiteKey] -> Component ;
    MkSection : Anchor -> String -> [Block] -> Section ;
    -- status vocabulary (the corpus's five statuses)
    StatusBuilt : Status ;
    StatusPartial : Status ;
    StatusAdoptedUnbuilt : Status ;
    StatusDesignPass : Status ;
    StatusDormant : Status ;
    -- blocks
    ProseBlock : [Inline] -> Block ;
    DefinitionBlock : Anchor -> Term -> [Inline] -> Block ;
    GrammarBlock : [Production] -> Block ;
    MkProduction : String -> String -> Production ;
    JudgementsBlock : String -> [MathRow] -> Block ;
    MkMathRow : String -> MathRow ;
    RuleBlock : Anchor -> String -> [MathRow] -> MathRow -> Block ;
    InventoryBlock : String -> Row -> [Row] -> Block ;
    MkHeaderRow : [Cell] -> Row ;
    MkBodyRow : [Cell] -> Row ;
    MkCell : [Inline] -> Cell ;
    RegisterBlock : [Item] -> Block ;
    PlainRegisterBlock : [Item] -> Block ;
    MkItem : String -> [Inline] -> Item ;
    MkPlainItem : [Inline] -> Item ;
    ApiCodeBlock : String -> String -> Block ;
    PlainCodeBlock : String -> String -> Block ;
    ExpectCodeBlock : String -> String -> String -> Block ;
    DiagramBlock : Anchor -> String -> CiteKey -> String -> Block ;
    ExampleBlock : String -> [Block] -> Block ;
    -- inlines
    Txt : String -> Inline ;
    Bold : [Inline] -> Inline ;
    Italic : [Inline] -> Inline ;
    TermRef : Term -> Inline ;
    TermDef : Term -> String -> Inline ;
    CiteRef : CiteKey -> Inline ;
    XRef : Anchor -> Inline ;
    MathInline : String -> Inline ;
    -- glue boundary: a punctuation-leading Text binds to its left neighbor
    ConsInlineGlued : Inline -> [Inline] -> [Inline] ;
}
