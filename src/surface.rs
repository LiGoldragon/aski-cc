#![allow(non_snake_case)]
//! Surface DB — the parsed aski AST in relational form.
//!
//! PascalCase relations (aski naming: nouns are types).
//! This is the FULL v0.9 syntax — richer than Kernel Aski.
//! Grammar rules transform Surface -> Kernel.
//!
//! The Kernel schema lives in aski-core. Surface extends it with
//! Surface-only relations (modules, grammar rules, supertraits, etc.).
//! Ascent structs cannot inherit, so shared relations are duplicated
//! here with matching shapes.

use ascent::ascent;

ascent! {
    pub struct Surface;

    // ── Nodes ──
    // Every parsed item gets a node.
    // Shape matches aski-core::World::Node exactly (with spans).
    relation Node(i64, String, String, Option<i64>, usize, usize, Option<i64>);
    // (id, kind, name, parent_id, span_start, span_end, scope_id)
    // kind: "domain", "struct", "trait_sig", "trait_impl", "method", "tail_method",
    //       "method_sig", "const", "main", "type_alias", "grammar_rule", "module"

    // ── Module headers (Surface only) ──
    relation ModuleExport(i64, String);
    // (module_id, exported_name)

    relation ModuleImport(i64, String, String);
    // (module_id, source_module, imported_name)

    // ── Domains ──
    // Shape matches aski-core::World::Variant
    relation Variant(i64, i64, String, Option<String>);
    // (domain_id, ordinal, name, wraps_type)

    // ── Structs ──
    // Shape matches aski-core::World::Field
    relation Field(i64, i64, String, String);
    // (struct_id, ordinal, name, type_ref)

    // ── Methods ──
    // Shape matches aski-core::World::Param
    relation Param(i64, i64, String, Option<String>, Option<String>);
    // (method_id, ordinal, kind, name, type_ref)
    // kind: "borrow_self", "mut_borrow_self", "owned_self", "owned", "named", "borrow", "mut_borrow"

    // Shape matches aski-core::World::Returns
    relation Returns(i64, String);
    // (method_id, type_ref)

    // ── Trait system ──
    // Shape matches aski-core::World::TraitImpl
    relation TraitImpl(String, String, i64);
    // (trait_name, type_name, impl_node_id)

    // Surface-only: supertrait relationships
    relation Supertrait(String, String);
    // (trait_name, supertrait_name)

    // Surface-only: trait bounds on type parameters
    relation TraitBound(i64, String);
    // (node_id, bound_expr) — e.g., "a&display"

    // Surface-only: associated types in trait impls
    relation AssociatedType(i64, String, Option<String>);
    // (impl_id, name, concrete_type)

    // Surface-only: associated constants in trait impls
    relation AssociatedConst(i64, String, String, Option<String>);
    // (impl_id, name, type_ref, value)

    // ── Constants ──
    // Shape matches aski-core::World::Constant
    relation Constant(i64, String, String, bool);
    // (node_id, name, type_ref, has_value)

    // ── Expressions ──
    // Shape matches aski-core::World::Expr
    relation Expr(i64, Option<i64>, String, i64, Option<String>);
    // (id, parent_id, kind, ordinal, value)

    // ── Match arms ──
    // Shape matches aski-core::World::MatchArm
    relation MatchArm(i64, i64, String, Option<i64>, String);
    // (match_id, ordinal, patterns_json, body_expr_id, arm_kind)

    // ── Type aliases ──
    relation TypeAlias(i64, String, String);
    // (id, name, aliased_type)

    // ── Grammar rules (Surface only — not in Kernel) ──
    relation GrammarRule(i64, String);
    // (id, rule_name) — grammar rules defined with <Name> [...]

    relation GrammarArm(i64, i64, String, String);
    // (rule_id, ordinal, pattern, result_expr)

    // ── Derived relations ──

    // Type containment — auto-derived from struct fields and domain variant wraps.
    // Matches aski-core::World::ContainedType derivation rules.
    relation ContainedType(String, String);
    // (parent_type, child_type) — immediate containment

    ContainedType(parent_type, field_type) <--
        Node(parent_id, kind, parent_type, _, _, _, _),
        if kind == "struct",
        Field(*parent_id, _, _, field_type);

    ContainedType(parent_type, field_type.clone()) <--
        Node(parent_id, kind, parent_type, _, _, _, _),
        if kind == "domain",
        Variant(*parent_id, _, _, wraps),
        if wraps.is_some(),
        let field_type = wraps.as_ref().unwrap();

    // Transitive closure for recursive type detection
    relation RecursiveType(String, String);
    RecursiveType(x, y) <-- ContainedType(x, y);
    RecursiveType(x, z) <-- ContainedType(x, y), RecursiveType(y, z);

    // Method ownership: which methods belong to which type
    relation MethodOwner(i64, String);
    // (method_id, type_name)

    MethodOwner(method_id, type_name) <--
        Node(method_id, kind, _, parent_opt, _, _, _),
        if kind == "method" || kind == "tail_method",
        if parent_opt.is_some(),
        let pid = parent_opt.as_ref().unwrap(),
        Node(pid, _, type_name, _, _, _, _);

    // Trait method coverage: which traits are fully implemented for a type.
    // TODO: Derivation requires checking that every method_sig in the trait
    // has a corresponding method in the impl body. Left for a future pass.
    relation TraitComplete(String, String);
    // (trait_name, type_name) — all required methods present
}

/// Create a new empty Surface DB
pub fn create() -> Surface {
    Surface::default()
}

/// Run derived rules after all inserts
pub fn resolve(surface: &mut Surface) {
    surface.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_type_detection() {
        let mut db = create();

        // Insert nodes for two structs (now with span fields)
        db.Node.push((0, "struct".into(), "Tree".into(), None, 0, 10, None));
        db.Node.push((1, "struct".into(), "Branch".into(), None, 11, 20, None));

        // Tree has a field of type Branch
        db.Field.push((0, 0, "branches".into(), "Branch".into()));
        // Branch has a field of type Tree (recursive)
        db.Field.push((1, 0, "subtree".into(), "Tree".into()));

        resolve(&mut db);

        // ContainedType is now auto-derived from Field + Node
        assert!(db.ContainedType.contains(&("Tree".into(), "Branch".into())));
        assert!(db.ContainedType.contains(&("Branch".into(), "Tree".into())));

        // Transitive: Tree -> Branch -> Tree
        assert!(db.RecursiveType.contains(&("Tree".into(), "Tree".into())));
        // Transitive: Branch -> Tree -> Branch
        assert!(db.RecursiveType.contains(&("Branch".into(), "Branch".into())));
    }

    #[test]
    fn empty_surface_resolves() {
        let mut db = create();
        resolve(&mut db);
        assert!(db.Node.is_empty());
        assert!(db.RecursiveType.is_empty());
    }

    #[test]
    fn linear_containment_chain() {
        let mut db = create();

        // A contains B, B contains C — no cycle
        db.Node.push((0, "struct".into(), "A".into(), None, 0, 5, None));
        db.Node.push((1, "struct".into(), "B".into(), None, 6, 10, None));
        db.Node.push((2, "struct".into(), "C".into(), None, 11, 15, None));
        db.Field.push((0, 0, "b".into(), "B".into()));
        db.Field.push((1, 0, "c".into(), "C".into()));

        resolve(&mut db);

        assert!(db.RecursiveType.contains(&("A".into(), "B".into())));
        assert!(db.RecursiveType.contains(&("A".into(), "C".into())));
        assert!(db.RecursiveType.contains(&("B".into(), "C".into())));

        // No reverse paths
        assert!(!db.RecursiveType.contains(&("C".into(), "A".into())));
        assert!(!db.RecursiveType.contains(&("C".into(), "B".into())));
    }

    #[test]
    fn method_owner_derived() {
        let mut db = create();

        // impl body node for type "Point"
        db.Node.push((1, "impl_body".into(), "Point".into(), None, 0, 50, None));
        db.Node.push((2, "method".into(), "distance".into(), Some(1), 5, 20, None));
        db.Node.push((3, "tail_method".into(), "scale".into(), Some(1), 21, 40, None));

        resolve(&mut db);

        assert!(db.MethodOwner.contains(&(2, "Point".into())));
        assert!(db.MethodOwner.contains(&(3, "Point".into())));
    }

    #[test]
    fn domain_variant_containment() {
        let mut db = create();

        db.Node.push((0, "domain".into(), "Expr".into(), None, 0, 20, None));
        db.Node.push((1, "struct".into(), "Term".into(), None, 21, 30, None));
        db.Variant.push((0, 0, "Lit".into(), Some("Term".into())));
        db.Variant.push((0, 1, "Empty".into(), None));

        resolve(&mut db);

        assert!(db.ContainedType.contains(&("Expr".into(), "Term".into())));
        // Empty has no wraps — should not create containment
        assert_eq!(db.ContainedType.len(), 1);
    }
}
