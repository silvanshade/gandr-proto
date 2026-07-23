abstract GandrDocs = {
  flags startcat = Component ;
  cat
    Component ; Section ; Block ; Inline ;
    Status ; SectionStatus ; Anchor ;
    Term ; CiteKey ;
    ListOrder ;
    Production ; MathRow ; Cell ; Row ; Item ;
    [Section] ; [Block] ; [Inline] ; [Production] ; [MathRow] ; [Cell] ; [Row] ; [Item] ; [CiteKey] ; [Anchor] ;
  fun
    -- component and sections
    MkComponent : Anchor -> String -> Status -> [Anchor] -> [Anchor] -> [Section] -> [CiteKey] -> Component ;
    MkSection : Anchor -> String -> SectionStatus -> [Block] -> Section ;
    -- a section nested inside another section's block list
    NestedSection : Section -> Block ;
    -- status vocabulary (the corpus's five statuses)
    StatusBuilt : Status ;
    StatusPartial : Status ;
    StatusAdoptedUnbuilt : Status ;
    StatusDesignPass : Status ;
    StatusDormant : Status ;
    -- section status: absent (inherit the component's) or an explicit override
    InheritSectionStatus : SectionStatus ;
    WithSectionStatus : Status -> SectionStatus ;
    -- blocks
    ProseBlock : [Inline] -> Block ;
    DefinitionBlock : Anchor -> Term -> [Inline] -> Block ;
    GrammarBlock : [Production] -> Block ;
    MkProduction : String -> String -> Production ;
    JudgementsBlock : String -> [MathRow] -> Block ;
    MkMathRow : String -> MathRow ;
    RuleBlock : Anchor -> String -> [MathRow] -> MathRow -> Block ;
    InventoryBlock : String -> Row -> [Row] -> Block ;
    -- semantic table payloads (proposal section 3.1; same shape, distinct
    -- semantics, distinct linearization classes)
    StagingPlanBlock : String -> Row -> [Row] -> Block ;
    DecisionTableBlock : String -> Row -> [Row] -> Block ;
    MkHeaderRow : [Cell] -> Row ;
    MkBodyRow : [Cell] -> Row ;
    MkCell : [Inline] -> Cell ;
    RegisterBlock : ListOrder -> [Item] -> Block ;
    PlainRegisterBlock : ListOrder -> [Item] -> Block ;
    -- list ordering (ordered lists linearize to <ol>, unordered to <ul>)
    OrderedList : ListOrder ;
    UnorderedList : ListOrder ;
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
    CodeInline : String -> Inline ;
    -- glue boundary: a punctuation-leading Text binds to its left neighbor
    ConsInlineGlued : Inline -> [Inline] -> [Inline] ;
}
