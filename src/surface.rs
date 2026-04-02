#![allow(non_snake_case)]
//! Surface DB — the parsed aski AST in relational form.
//!
//! PascalCase relations (aski naming: nouns are types).
//! This is the FULL v0.9 syntax — richer than Kernel Aski.
//! Grammar rules transform Surface → Kernel.

use ascent::ascent;

ascent! {
    pub struct Surface;

    // ── Nodes ──
    // Every parsed item gets a node
    relation Node(i64, String, String, Option<i64>);
    // (id, kind, name, parent_id)
    // kind: "domain", "struct", "trait_sig", "trait_impl", "method", "tail_method",
    //       "method_sig", "const", "main", "type_alias", "grammar_rule", "module"

    // ── Module headers ──
    relation ModuleExport(i64, String);
    // (module_id, exported_name)

    relation ModuleImport(i64, String, String);
    // (module_id, source_module, imported_name)

    // ── Domains ──
    relation Variant(i64, i64, String, Option<String>);
    // (domain_id, ordinal, name, wraps_type)

    // ── Structs ──
    relation Field(i64, i64, String, String);
    // (struct_id, ordinal, name, type_ref)

    // ── Methods ──
    relation Param(i64, i64, String, Option<String>, Option<String>);
    // (method_id, ordinal, kind, name, type_ref)
    // kind: "borrow_self", "mut_borrow_self", "owned_self", "owned", "named", "borrow", "mut_borrow"

    relation Returns(i64, String);
    // (method_id, type_ref)

    // ── Trait system ──
    relation TraitImpl(String, String, i64);
    // (trait_name, type_name, impl_node_id)

    relation Supertrait(String, String);
    // (trait_name, supertrait_name)

    relation TraitBound(i64, String);
    // (node_id, bound_expr) — e.g., "a&display"

    relation AssociatedType(i64, String, Option<String>);
    // (impl_id, name, concrete_type)

    relation AssociatedConst(i64, String, String, Option<String>);
    // (impl_id, name, type_ref, value)

    // ── Constants ──
    relation Constant(i64, String, String, bool);
    // (node_id, name, type_ref, has_value)

    // ── Expressions ──
    relation Expr(i64, Option<i64>, String, i64, Option<String>);
    // (id, parent_id, kind, ordinal, value)

    // ── Match arms ──
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

    // Type containment (struct field → type)
    relation ContainedType(String, String);

    // Transitive closure for recursive type detection
    relation RecursiveType(String, String);
    RecursiveType(x, y) <-- ContainedType(x, y);
    RecursiveType(x, z) <-- ContainedType(x, y), RecursiveType(y, z);

    // Method ownership: which methods belong to which type
    relation MethodOwner(i64, String);
    // (method_id, type_name)

    // Trait method coverage: which traits are fully implemented for a type
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

        // Insert nodes for two structs
        db.Node.push((0, "struct".into(), "Tree".into(), None));
        db.Node.push((1, "struct".into(), "Branch".into(), None));

        // Tree has a field of type Branch
        db.Field.push((0, 0, "branches".into(), "Branch".into()));
        // Branch has a field of type Tree (recursive)
        db.Field.push((1, 0, "subtree".into(), "Tree".into()));

        // Populate ContainedType from fields
        db.ContainedType.push(("Tree".into(), "Branch".into()));
        db.ContainedType.push(("Branch".into(), "Tree".into()));

        resolve(&mut db);

        // Direct containment
        assert!(db.RecursiveType.contains(&("Tree".into(), "Branch".into())));
        assert!(db.RecursiveType.contains(&("Branch".into(), "Tree".into())));

        // Transitive: Tree → Branch → Tree
        assert!(db.RecursiveType.contains(&("Tree".into(), "Tree".into())));
        // Transitive: Branch → Tree → Branch
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
        db.ContainedType.push(("A".into(), "B".into()));
        db.ContainedType.push(("B".into(), "C".into()));

        resolve(&mut db);

        assert!(db.RecursiveType.contains(&("A".into(), "B".into())));
        assert!(db.RecursiveType.contains(&("A".into(), "C".into())));
        assert!(db.RecursiveType.contains(&("B".into(), "C".into())));

        // No reverse paths
        assert!(!db.RecursiveType.contains(&("C".into(), "A".into())));
        assert!(!db.RecursiveType.contains(&("C".into(), "B".into())));
    }
}
