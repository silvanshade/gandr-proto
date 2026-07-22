abstract GandrDocsLex = GandrDocs ** {
  fun
    -- terms (generated from the corpus term registry; PoC: hand-written)
    term_component : Term ;
    term_status : Term ;
    term_prose : Term ;
    -- cite keys (generated from refs.yml; PoC: hand-written)
    cite_P_2 : CiteKey ;
    cite_A_1a : CiteKey ;
    -- anchors (generated per component; PoC: component-vocabulary.xml)
    anchor_component_vocabulary : Anchor ;
    anchor_spec_index : Anchor ;
    anchor_cv_overview : Anchor ;
    anchor_cv_blocks : Anchor ;
    anchor_cv_examples : Anchor ;
    anchor_cv_def_prose : Anchor ;
    anchor_cv_rule_app : Anchor ;
    anchor_cv_diagram : Anchor ;
}
