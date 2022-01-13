use std::convert::TryInto;

use homotopy_common::idx::IdxVec;
use petgraph::{graph::NodeIndex, visit::EdgeRef, Graph};

use crate::{
    common::{Boundary, DimensionError, Height, SliceIndex},
    diagram::{Diagram, DiagramN},
    rewrite::{Rewrite, RewriteN},
};

pub type Coord = Vec<SliceIndex>;

pub fn add_coord(coord: SliceIndex, mut acc: Coord) -> Option<Coord> {
    acc.push(coord);
    Some(acc)
}

pub type SliceGraph<NodeKey, EdgeKey> = Graph<(NodeKey, Diagram), (EdgeKey, Rewrite)>;

pub struct GraphBuilder;

impl GraphBuilder {
    pub fn build(diagram: Diagram, depth: usize) -> Result<SliceGraph<Coord, ()>, DimensionError> {
        if depth > diagram.dimension() {
            return Err(DimensionError);
        }

        let mut graph = Graph::new();
        graph.add_node((vec![], diagram));

        for _ in 0..depth {
            graph = explode(&graph, add_coord, |_, _| Some(()))?.0;
        }

        Ok(graph)
    }
}

/// Describes from where a rewrite in the output of explosion originates.
pub enum RewriteOrigin {
    /// From a diagram's cospans.
    Internal,
    /// Sparse identity from a rewrite.
    Sparse,
    /// Cone regular slices from a rewrite.
    RegularSlice,
    /// Cone singular `slices` (slice `x` of `y`) from a rewrite.
    SingularSlice(usize, usize),
}

pub fn explode<NodeKey, EdgeKey, NodeMap, EdgeMap>(
    graph: &SliceGraph<NodeKey, EdgeKey>,
    node_map: NodeMap,
    edge_map: EdgeMap,
) -> Result<
    (
        SliceGraph<NodeKey, EdgeKey>,
        IdxVec<NodeIndex, Vec<Option<NodeIndex>>>,
    ),
    DimensionError,
>
where
    NodeKey: Clone,
    EdgeKey: Clone,
    NodeMap: Fn(SliceIndex, NodeKey) -> Option<NodeKey>,
    EdgeMap: Fn(RewriteOrigin, Option<EdgeKey>) -> Option<EdgeKey>,
{
    use Height::{Regular, Singular};

    let mut exploded_graph: SliceGraph<NodeKey, EdgeKey> = Graph::new();

    // Maps every node in the original graph to its slices in the exploded graph.
    let mut node_to_slices: IdxVec<NodeIndex, Vec<Option<NodeIndex>>> =
        IdxVec::with_capacity(graph.node_count());

    for (key, diagram) in graph.node_weights() {
        let diagram: &DiagramN = diagram.try_into()?;

        let mut slices = Vec::with_capacity(diagram.size() * 2 + 3);

        // Source slice
        slices.push((
            node_map(Boundary::Source.into(), key.clone()),
            diagram.source(),
        ));

        // Interior slices
        for (i, slice) in diagram.slices().enumerate() {
            slices.push((node_map(Height::from(i).into(), key.clone()), slice));
        }

        // Target slice
        slices.push((
            node_map(Boundary::Target.into(), key.clone()),
            diagram.target(),
        ));

        let nodes: Vec<Option<NodeIndex>> = slices
            .into_iter()
            .map(|(k, d)| k.map(|i| exploded_graph.add_node((i, d))))
            .collect();

        // Identity rewrite from source slice
        if let (Some(s), Some(t)) = (nodes[0], nodes[1]) {
            if let Some(key) = edge_map(RewriteOrigin::Internal, None) {
                exploded_graph.add_edge(s, t, (key, Rewrite::identity(diagram.dimension() - 1)));
            }
        }

        // Rewrites between interior slices
        for (i, cospan) in diagram.cospans().iter().enumerate() {
            if let Some(singular) = nodes[usize::from(Singular(i)) + 1] {
                if let Some(regular) = nodes[usize::from(Regular(i)) + 1] {
                    if let Some(key) = edge_map(RewriteOrigin::Internal, None) {
                        exploded_graph.add_edge(regular, singular, (key, cospan.forward.clone()));
                    }
                }

                if let Some(regular) = nodes[usize::from(Regular(i + 1)) + 1] {
                    if let Some(key) = edge_map(RewriteOrigin::Internal, None) {
                        exploded_graph.add_edge(regular, singular, (key, cospan.backward.clone()));
                    }
                }
            }
        }

        // Identity rewrite from target slice
        if let (Some(s), Some(t)) = (nodes[diagram.size() * 2 + 2], nodes[diagram.size() * 2 + 1]) {
            if let Some(key) = edge_map(RewriteOrigin::Internal, None) {
                exploded_graph.add_edge(s, t, (key, Rewrite::identity(diagram.dimension() - 1)));
            }
        }

        node_to_slices.push(nodes);
    }

    for e in graph.edge_references() {
        let rewrite: &RewriteN = (&e.weight().1).try_into()?;

        let source_slices = &node_to_slices[e.source()];
        let source_size = source_slices.len();
        let target_slices = &node_to_slices[e.target()];
        let target_size = target_slices.len();

        // Identity rewrite between source slices
        if let (Some(s), Some(t)) = (source_slices[0], target_slices[0]) {
            if let Some(key) = edge_map(RewriteOrigin::Sparse, Some(e.weight().0.clone())) {
                exploded_graph.add_edge(s, t, (key, Rewrite::identity(rewrite.dimension() - 1)));
            }
        }

        // Identity rewrite between target slices
        if let (Some(s), Some(t)) = (
            source_slices[source_size - 1],
            target_slices[target_size - 1],
        ) {
            if let Some(key) = edge_map(RewriteOrigin::Sparse, Some(e.weight().0.clone())) {
                exploded_graph.add_edge(s, t, (key, Rewrite::identity(rewrite.dimension() - 1)));
            }
        }

        // Rewrite slices targeting singular levels
        let mut source_height = 0;
        while source_height < (source_size - 3) / 2 {
            let target_height = rewrite.singular_image(source_height);
            if let (Some(s), Some(t)) = (
                source_slices[usize::from(Singular(source_height)) + 1],
                target_slices[usize::from(Singular(target_height)) + 1],
            ) {
                if let Some(cone) = rewrite.cone_over_target(target_height) {
                    for (i, singular) in cone.internal.slices.iter().enumerate() {
                        if let Some(key) = edge_map(
                            RewriteOrigin::SingularSlice(i, cone.len()),
                            Some(e.weight().0.clone()),
                        ) {
                            if let Some(r) =
                                source_slices[usize::from(Singular(source_height + i)) + 1]
                            {
                                exploded_graph.add_edge(r, t, (key, singular.clone()));
                            }
                        }
                    }
                    source_height += cone.len();
                } else {
                    if let Some(key) = edge_map(RewriteOrigin::Sparse, Some(e.weight().0.clone())) {
                        exploded_graph.add_edge(
                            s,
                            t,
                            (key, Rewrite::identity(rewrite.dimension() - 1)),
                        );
                    }
                    source_height += 1;
                }
            }
        }

        // Rewrite slices targeting regular levels (identities)
        for target_height in 0..(target_size - 1) / 2 {
            let source_height = rewrite.regular_image(target_height);
            if let (Some(s), Some(t)) = (
                source_slices[usize::from(Regular(source_height)) + 1],
                target_slices[usize::from(Regular(target_height)) + 1],
            ) {
                if let Some(key) = edge_map(RewriteOrigin::Sparse, Some(e.weight().0.clone())) {
                    exploded_graph.add_edge(
                        s,
                        t,
                        (key, Rewrite::identity(rewrite.dimension() - 1)),
                    );
                }
            }
        }

        // Rewrite slices from regular levels targeting singular levels
        // Between singular slices
        for source_height in 0..(source_size - 1) / 2 {
            let preimage = rewrite.regular_preimage(source_height);
            if preimage.is_empty() {
                // regular slice between two singular slices
                let target_height = preimage.start;
                if let (Some(s), Some(t)) = (
                    source_slices[usize::from(Regular(source_height)) + 1],
                    target_slices[usize::from(Singular(target_height)) + 1],
                ) {
                    if let Some(key) =
                        edge_map(RewriteOrigin::RegularSlice, Some(e.weight().0.clone()))
                    {
                        exploded_graph.add_edge(
                            s,
                            t,
                            (
                                key,
                                DiagramN::try_from(graph[e.source()].1.clone())?.cospans()
                                    [source_height]
                                    .forward
                                    .compose(&rewrite.slice(Singular(source_height)))
                                    .unwrap(),
                            ),
                        );
                    }
                }
            }
        }
        // Empty cone case
        for target_height in 0..(target_size - 3) / 2 {
            let preimage = rewrite.singular_preimage(target_height);
            if preimage.is_empty() {
                let source_height = preimage.start;
                if let (Some(s), Some(t)) = (
                    source_slices[usize::from(Regular(source_height)) + 1],
                    target_slices[usize::from(Singular(target_height)) + 1],
                ) {
                    if let Some(key) =
                        edge_map(RewriteOrigin::RegularSlice, Some(e.weight().0.clone()))
                    {
                        exploded_graph.add_edge(
                            s,
                            t,
                            (
                                key,
                                DiagramN::try_from(graph[e.target()].1.clone())?.cospans()
                                    [target_height]
                                    .forward
                                    .clone(),
                            ),
                        );
                    }
                }
            }
        }
    }

    Ok((exploded_graph, node_to_slices))
}
