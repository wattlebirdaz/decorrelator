use super::prelude::*;
use crate::col_id_generator::ColIdGeneratorRef;
use crate::rules::{Rule, RulesRef};

impl RelExpr {
    pub fn map(
        self,
        optimize: bool,
        enabled_rules: &RulesRef,
        col_id_gen: &ColIdGeneratorRef,
        exprs: impl IntoIterator<Item = (usize, Expr)>,
    ) -> RelExpr {
        let mut exprs: Vec<(usize, Expr)> = exprs.into_iter().collect();

        if exprs.is_empty() {
            return self;
        }

        if optimize {
            if enabled_rules.is_enabled(&Rule::Hoist) {
                for i in 0..exprs.len() {
                    // Only hoist expressions with subqueries
                    if exprs[i].1.has_subquery() {
                        let (id, expr) = exprs.swap_remove(i);
                        return self.map(true, enabled_rules, col_id_gen, exprs).hoist(
                            enabled_rules,
                            col_id_gen,
                            id,
                            expr,
                        );
                    }
                }
            }

            match self {
                RelExpr::Map {
                    input,
                    exprs: mut existing_exprs,
                } => {
                    // We can combine two maps into one by merging the expressions.
                    // FROM: map(@2 <- @1) <- map(@1 <- @0 + 5)
                    // TO: map(@2 <- @0 + 5, @1 <- @0 + 5)
                    // If there is a subquery, then it should have already been hoisted.
                    #[cfg(debug_assertions)]
                    {
                        // Check that none of the expressions have subqueries
                        for (_, expr) in &existing_exprs {
                            assert!(!expr.has_subquery());
                        }
                        for (_, expr) in &exprs {
                            assert!(!expr.has_subquery());
                        }
                    }

                    // Merge the expressions
                    for (dest, mut expr) in exprs {
                        expr = expr.replace_variables_with_exprs(
                            &existing_exprs.iter().cloned().collect(),
                        );
                        existing_exprs.push((dest, expr));
                    }
                    input.map(true, enabled_rules, col_id_gen, existing_exprs)
                }
                _ => self.map(false, enabled_rules, col_id_gen, exprs),
            }
        } else {
            RelExpr::Map {
                input: Box::new(self),
                exprs,
            }
        }
    }
}
