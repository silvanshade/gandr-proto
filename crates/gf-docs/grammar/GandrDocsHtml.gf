concrete GandrDocsHtml of GandrDocs = {
  lincat
    Component , Section , Block , Inline , Production , MathRow , Item , Status = Str ;
    Cell = { th , td : Str } ;
    [Cell] = { th , td : Str } ;
    Row = Str ;
    Term = { key , text : Str } ;
    Anchor = { id , title : Str } ;
    CiteKey = { key : Str } ;
  lin
    MkComponent a t st _grounds _der secs refs =
      "<h1>" ++ "​" ++ t.s ++ "​" ++ "</h1>" ++
      "<p class=\"status-chip status-" ++ "​" ++ st ++ "​" ++ "\">" ++ "​" ++ st ++ "​" ++ "</p>" ++
      secs.s ++
      "<h2>References</h2><ul class=\"refs\">" ++ "​" ++ refs.s ++ "​" ++ "</ul>" ;
    MkSection a t bs = "<section id=\"" ++ "​" ++ a.id ++ "​" ++ "\"><h2>" ++ "​" ++ t.s ++ "​" ++ "</h2>" ++ bs.s ++ "​" ++ "</section>" ;
    StatusBuilt = "built" ;
    StatusPartial = "partial" ;
    StatusAdoptedUnbuilt = "adopted-unbuilt" ;
    StatusDesignPass = "design-pass" ;
    StatusDormant = "dormant" ;
    ProseBlock xs = "<p>" ++ "​" ++ xs.s ++ "​" ++ "</p>" ;
    DefinitionBlock a t xs =
      "<div class=\"definition\" id=\"" ++ "​" ++ a.id ++ "​" ++ "\"><p><strong>Definition</strong> (<dfn id=\"term-" ++ "​" ++ t.key ++ "​" ++ "\">" ++ "​" ++ t.text ++ "​" ++ "</dfn>). " ++ xs.s ++ "​" ++ "</p></div>" ;
    GrammarBlock ps = "<dl class=\"grammar\">" ++ ps.s ++ "​" ++ "</dl>" ;
    MkProduction sym body = "<dt>" ++ "​" ++ sym.s ++ "​" ++ "</dt><dd class=\"mono\">" ++ "​" ++ body.s ++ "​" ++ "</dd>" ;
    JudgementsBlock t rows = "<div class=\"judgements\"><h3>" ++ "​" ++ t.s ++ "​" ++ "</h3>" ++ rows.s ++ "​" ++ "</div>" ;
    MkMathRow src = "<div><span class=\"math math-block\">" ++ "​" ++ src.s ++ "​" ++ "</span></div>" ;
    RuleBlock a name prem conc =
      "<div class=\"rule\" id=\"" ++ "​" ++ a.id ++ "​" ++ "\"><h3>" ++ "​" ++ name.s ++ "​" ++ "</h3>" ++ prem.s ++ "<div class=\"conclusion\">" ++ conc ++ "​" ++ "</div></div>" ;
    InventoryBlock cap hdr rows =
      "<figure class=\"table\"><table><thead>" ++ hdr ++ "</thead><tbody>" ++ rows.s ++ "</tbody></table><figcaption>" ++ "​" ++ cap.s ++ "​" ++ "</figcaption></figure>" ;
    MkHeaderRow cs = "<tr>" ++ "​" ++ cs.th ++ "​" ++ "</tr>" ;
    MkBodyRow cs = "<tr>" ++ "​" ++ cs.td ++ "​" ++ "</tr>" ;
    MkCell xs = { th = "<th>" ++ "​" ++ xs.s ++ "​" ++ "</th>" ; td = "<td>" ++ "​" ++ xs.s ++ "​" ++ "</td>" } ;
    RegisterBlock items = "<ol>" ++ "​" ++ items.s ++ "​" ++ "</ol>" ;
    PlainRegisterBlock items = "<ul>" ++ "​" ++ items.s ++ "​" ++ "</ul>" ;
    MkItem lead xs = "<li><strong>" ++ "​" ++ lead.s ++ "​" ++ "</strong> — " ++ xs.s ++ "​" ++ "</li>" ;
    MkPlainItem xs = "<li>" ++ "​" ++ xs.s ++ "​" ++ "</li>" ;
    ApiCodeBlock lang payload = "<pre><code class=\"lang-" ++ "​" ++ lang.s ++ " api\">" ++ "​" ++ payload.s ++ "​" ++ "</code></pre>" ;
    PlainCodeBlock lang payload = "<pre><code class=\"lang-" ++ "​" ++ lang.s ++ "​" ++ "\">" ++ "​" ++ payload.s ++ "​" ++ "</code></pre>" ;
    ExpectCodeBlock lang expect payload =
      "<pre><code class=\"lang-" ++ "​" ++ lang.s ++ "\">" ++ "​" ++ payload.s ++ "​" ++ "</code></pre><p class=\"expect\">expected output:" ++ expect.s ++ "​" ++ "</p>" ;
    DiagramBlock a cap cite src =
      "<figure class=\"diagram\" id=\"" ++ "​" ++ a.id ++ "​" ++ "\"><div class=\"diagram-slot\" data-cites=\"" ++ "​" ++ cite.key ++ "​" ++ "\">" ++ src.s ++ "​" ++ "</div><figcaption>" ++ "​" ++ cap.s ++ "​" ++ "</figcaption></figure>" ;
    ExampleBlock t bs = "<div class=\"example\"><h3>Example:" ++ t.s ++ "​" ++ "</h3>" ++ bs.s ++ "​" ++ "</div>" ;
    Txt s = s.s ;
    Bold xs = "<strong>" ++ "​" ++ xs.s ++ "​" ++ "</strong>" ;
    Italic xs = "<em>" ++ "​" ++ xs.s ++ "​" ++ "</em>" ;
    TermRef t = "<a class=\"term\" href=\"#term-" ++ "​" ++ t.key ++ "​" ++ "\">" ++ "​" ++ t.text ++ "​" ++ "</a>" ;
    TermDef t disp = "<dfn id=\"term-" ++ "​" ++ t.key ++ "​" ++ "\">" ++ "​" ++ disp.s ++ "​" ++ "</dfn>" ;
    CiteRef c = "<sup><a class=\"cite\" href=\"#ref-" ++ "​" ++ c.key ++ "​" ++ "\">[" ++ "​" ++ c.key ++ "​" ++ "]</a></sup>" ;
    XRef a = "<a class=\"xref\" href=\"#" ++ "​" ++ a.id ++ "​" ++ "\">" ++ "​" ++ a.title ++ "​" ++ "</a>" ;
    MathInline src = "<span class=\"math\">" ++ "​" ++ src.s ++ "​" ++ "</span>" ;
    -- list folds: word-spacing joins (++) inside prose-level lists, glue (BIND) elsewhere
    BaseSection = { s = "" } ;
    ConsSection x xs = { s = x ++ xs.s } ;
    BaseBlock = { s = "" } ;
    ConsBlock x xs = { s = x ++ xs.s } ;
    BaseInline = { s = "" } ;
    ConsInline x xs = { s = x ++ xs.s } ;
    ConsInlineGlued x xs = { s = x ++ "​" ++ xs.s } ;
    BaseProduction = { s = "" } ;
    ConsProduction x xs = { s = x ++ xs.s } ;
    BaseMathRow = { s = "" } ;
    ConsMathRow x xs = { s = x ++ xs.s } ;
    BaseCell = { th = "" ; td = "" } ;
    ConsCell x xs = { th = x.th ++ "​" ++ xs.th ; td = x.td ++ "​" ++ xs.td } ;
    BaseRow = { s = "" } ;
    ConsRow x xs = { s = x ++ xs.s } ;
    BaseItem = { s = "" } ;
    ConsItem x xs = { s = x ++ xs.s } ;
    BaseCiteKey = { s = "" } ;
    ConsCiteKey x xs = { s = "<li id=\"ref-" ++ "​" ++ x.key ++ "​" ++ "\">[" ++ "​" ++ x.key ++ "​" ++ "]</li>" ++ xs.s } ;
    BaseAnchor = { s = "" } ;
    ConsAnchor x xs = { s = x.id ++ xs.s } ;
}
