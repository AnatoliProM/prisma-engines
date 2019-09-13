use super::*;
use crate::query_graph::{EdgeRef, Node, NodeRef, QueryGraph};
use std::convert::TryInto;

pub struct Expressionista;

impl Expressionista {
    pub fn translate(mut graph: QueryGraph) -> QueryExecutionResult<Expression> {
        graph
            .root_nodes()
            .into_iter()
            .map(|root_node| Self::build_expression(&mut graph, &root_node, None))
            .collect::<QueryExecutionResult<Vec<Expression>>>()
            .map(|res| Expression::Sequence { seq: res })
    }

    fn build_expression(
        graph: &mut QueryGraph,
        node: &NodeRef,
        parent_edge: Option<EdgeRef>,
    ) -> QueryExecutionResult<Expression> {
        // todo children pointing to ancestor handling

        match graph.node_content(node).unwrap() {
            Node::Query(_) => Self::build_query_expression(graph, node, parent_edge),
            Node::Flow(_) => Self::build_flow_expression(graph, node, parent_edge),
        }
    }

    fn build_query_expression(
        graph: &mut QueryGraph,
        node: &NodeRef,
        parent_edge: Option<EdgeRef>,
    ) -> QueryExecutionResult<Expression> {
        // Child edges are ordered, evaluation order is low to high in the graph, unless other rules override.
        let mut direct_children = graph.direct_child_pairs(&node);

        // Find the positions of all result returning graph nodes.
        let mut result_positions: Vec<usize> = direct_children
            .iter()
            .enumerate()
            .filter_map(|(ix, (_, child_node))| {
                if graph.subgraph_contains_result(&child_node) {
                    Some(ix)
                } else {
                    None
                }
            })
            .collect();

        let mut result_scopes: Vec<Vec<(EdgeRef, NodeRef)>> = vec![];

        // Start splitting of at the end to keep indices intact.
        result_positions.reverse();
        result_positions.iter().for_each(|pos| {
            let scope = direct_children.split_off(*pos);
            result_scopes.push(scope);
        });

        // Because we split from right to left, everything remaining in `direct_children`
        // doesn't belong into results, and is executed before all result scopes.
        let mut expressions: Vec<Expression> = direct_children
            .into_iter()
            .map(|(edge, node)| Self::build_expression(graph, &node, Some(edge)))
            .collect::<QueryExecutionResult<Vec<Expression>>>()?;

        // Fold result scopes into one expression.
        if result_scopes.len() > 0 {
            let result_exp = Self::fold_result_scopes(graph, vec![], result_scopes)?;
            expressions.push(result_exp);
        }

        // --- children end ---

        let is_result = graph.is_result_node(&node);
        let node_id = node.id();
        let query: Query = graph.pluck_node(&node).try_into()?;
        let expr = Self::query_expression(graph, parent_edge, query);

        if expressions.is_empty() {
            Ok(expr)
        } else {
            let node_binding_name = format!("{}", node_id);

            // Add a final statement to the evaluation if the current node has child nodes and is supposed to be the
            // final result, to make sure it propagates upwards.
            if is_result {
                expressions.push(Expression::Get {
                    binding_name: node_binding_name.clone(),
                });
            }

            Ok(Expression::Let {
                bindings: vec![Binding {
                    name: node_binding_name,
                    expr,
                }],
                expressions,
            })
        }
    }

    fn query_expression(graph: &mut QueryGraph, parent_edge: Option<EdgeRef>, query: Query) -> Expression {
        match (parent_edge, query) {
            (None, query) => Expression::Query { query },

            (Some(child_edge), query) => {
                let parent_binding_name = graph.edge_source(&child_edge).id();
                match graph.pluck_edge(&child_edge) {
                    QueryGraphDependency::ParentId(f) => Expression::Func {
                        func: Box::new(move |env: Env| {
                            let parent_id = env.get(&parent_binding_name).and_then(|pid| pid.as_id());
                            let query = f(query.into(), parent_id);

                            Expression::Query {
                                query: query.try_into().unwrap(),
                            }
                        }),
                    },

                    QueryGraphDependency::ExecutionOrder => Expression::Query { query },

                    _ => unreachable!(),
                }
            }
        }
    }

    fn build_flow_expression(
        graph: &mut QueryGraph,
        node: &NodeRef,
        parent_edge: Option<EdgeRef>,
    ) -> QueryExecutionResult<Expression> {
        let flow: Flow = graph.pluck_node(node).try_into()?;

        match flow {
            Flow::If(_) => {
                let child_pairs = graph.child_pairs(node);

                // Graph validation guarantees this succeeds.
                let (mut then_pair, mut else_pair): (Vec<(EdgeRef, NodeRef)>, Vec<(EdgeRef, NodeRef)>) = child_pairs
                    .into_iter()
                    .partition(|(edge, _)| match graph.edge_content(&edge).unwrap() {
                        QueryGraphDependency::Then => true,
                        QueryGraphDependency::Else => false,
                        _ => unreachable!(),
                    });

                let then_pair = then_pair.pop().unwrap();
                let else_pair = else_pair.pop();

                // Build expressions for both arms. They are treated as separate root nodes.
                let then_expr = Self::build_expression(graph, &then_pair.1, None)?;
                let else_expr = else_pair
                    .into_iter()
                    .map(|(_, node)| Self::build_expression(graph, &node, None))
                    .collect::<QueryExecutionResult<Vec<Expression>>>()?;

                Ok(match parent_edge {
                    Some(p) => {
                        match graph.pluck_edge(&p) {
                            QueryGraphDependency::ParentId(f) => {
                                let parent_binding_name = graph.edge_source(&p).id();
                                Expression::Func {
                                    func: Box::new(move |env| {
                                        let parent_id = env.get(&parent_binding_name).and_then(|pid| pid.as_id());

                                        // Todo: This still needs some interface polishing...
                                        let flow: Flow = f(flow.into(), parent_id).try_into().unwrap();
                                        match flow {
                                            Flow::If(cond_fn) => {
                                                // Todo: Maybe this construct points to a misconception: That the if node needs to be actually in the program.
                                                // At this point here, we could evaluate the condition already and decide which branch to return.
                                                Expression::If {
                                                    func: cond_fn,
                                                    then: vec![then_expr],
                                                    else_: else_expr,
                                                }
                                            }
                                            _ => unreachable!(),
                                        }
                                    }),
                                }
                            }
                            _ => unimplemented!(),
                        }
                    }

                    None => unimplemented!(),
                })
            }
        }
    }

    // All result scopes are nested into each other to allow a final select at the end,
    // resulting in a single expression.
    fn fold_result_scopes(
        graph: &mut QueryGraph,
        mut result_binding_names: Vec<String>,
        mut result_scopes: Vec<Vec<(EdgeRef, NodeRef)>>,
    ) -> QueryExecutionResult<Expression> {
        let current_scope = result_scopes.split_off(0).pop().unwrap();
        let result_binding_name = current_scope.first().unwrap().1.id();
        let has_sub_scopes = result_scopes.len() > 0; // todo necessary?
        let has_parent_scopes = result_binding_names.len() > 0;

        result_binding_names.push(result_binding_name.clone());

        let mut expressions: Vec<Expression> = current_scope
            .into_iter()
            .map(|(edge, node)| Self::build_expression(graph, &node, Some(edge)))
            .collect::<QueryExecutionResult<Vec<Expression>>>()?;

        // The first expression of every result scope is the one returning a potential result.
        let head = expressions.split_off(0).pop().unwrap();

        // If there are more scopes, build the nested ones.
        let last_expression = if has_sub_scopes {
            Self::fold_result_scopes(graph, result_binding_names, result_scopes)
        // expressions.push(sub_scope);
        } else {
            // Else, build the final select if necessary
            Ok(Expression::GetFirstNonEmpty {
                binding_names: result_binding_names,
            })
        }?;

        // When to bind the result expression to a Let:
        // - Result node has child expressions.
        // - Current scope has more subscopes.
        // - Current scope has parent scopes.
        if expressions.len() > 0 || has_sub_scopes || has_parent_scopes {
            expressions.push(last_expression);

            Ok(Expression::Let {
                bindings: vec![Binding {
                    name: result_binding_name,
                    expr: head,
                }],
                expressions, // last_expression
            })
        } else {
            Ok(head)
        }
    }
}
