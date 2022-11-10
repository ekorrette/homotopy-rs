//! Functions to collapse diagram scaffolds; used in contraction, typechecking etc.
use std::{collections::BTreeSet, ops::Index, rc::Rc};

use homotopy_common::{declare_idx, hash::FastHashMap, idx::Idx};
use itertools::Itertools;
use once_cell::unsync::OnceCell;
use petgraph::{
    data::Build,
    prelude::DiGraph,
    stable_graph::{DefaultIx, EdgeIndex, IndexType, NodeIndex},
    unionfind::UnionFind,
    visit::{EdgeCount, EdgeRef, GraphBase, IntoEdgeReferences, IntoNodeReferences, Topo, Walker},
    Direction::{Incoming, Outgoing},
};

use crate::{
    label::Coord,
    scaffold::{Explodable, Scaffold, ScaffoldGraph, ScaffoldNode, StableScaffold},
    Diagram, Height, Rewrite0, SliceIndex,
};

/// Trait for objects which have associated coordinates in `C`.
pub(crate) trait Cartesian<C: Copy> {
    /// Return the coordinate of this object.
    fn coordinate(&self) -> &[C];
}

impl<C: Copy, T: Cartesian<C>> Cartesian<C> for &T {
    fn coordinate(&self) -> &[C] {
        (*self).coordinate()
    }
}

impl<C: Copy> Cartesian<C> for Vec<C> {
    fn coordinate(&self) -> &[C] {
        self.as_slice()
    }
}

#[derive(Clone)]
pub(crate) enum OneMany<T, TS>
where
    TS: IntoIterator<Item = T>,
{
    One(T),
    Many(TS),
}

impl<T, TS: IntoIterator<Item = T>> From<T> for OneMany<T, TS> {
    fn from(x: T) -> Self {
        Self::One(x)
    }
}

impl<T, TS> Default for OneMany<T, TS>
where
    T: Default,
    TS: IntoIterator<Item = T>,
{
    fn default() -> Self {
        Self::One(T::default())
    }
}

impl<T, TS> IntoIterator for OneMany<T, TS>
where
    TS: IntoIterator<Item = T> + FromIterator<T>,
{
    type Item = T;

    type IntoIter = <TS as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            OneMany::One(x) => std::iter::once(x).collect::<TS>().into_iter(),
            OneMany::Many(xs) => xs.into_iter(),
        }
    }
}

impl<C, T, TS> Cartesian<C> for OneMany<T, TS>
where
    C: Copy,
    T: Cartesian<C>,
    TS: IntoIterator<Item = T>,
{
    fn coordinate(&self) -> &[C] {
        if let Self::One(c) = self {
            c.coordinate()
        } else {
            unreachable!()
        }
    }
}

impl<T, TS> Extend<OneMany<T, TS>> for OneMany<T, TS>
where
    T: Clone,
    TS: IntoIterator<Item = T> + FromIterator<T> + Extend<T>,
{
    fn extend<U: IntoIterator<Item = OneMany<T, TS>>>(&mut self, iter: U) {
        match self {
            Self::One(x) => {
                *self = Self::Many(
                    std::iter::once(x.clone())
                        .chain(
                            iter.into_iter()
                                .flat_map(std::iter::IntoIterator::into_iter),
                        )
                        .collect(),
                );
            }
            Self::Many(xs) => xs.extend(
                iter.into_iter()
                    .flat_map(std::iter::IntoIterator::into_iter),
            ),
        }
    }
}

/// Helper function to unify two nodes within a stable graph.
///
/// # Panics
///
/// Panics if `graph` edges are not 0-rewrites.
pub(crate) fn unify<V, E, Ix>(
    graph: &mut StableScaffold<V, E, Ix>,
    p: NodeIndex<Ix>,
    q: NodeIndex<Ix>,
    quotient: &mut UnionFind<NodeIndex<Ix>>,
    mut on_remove_node: impl FnMut(NodeIndex<Ix>),
    mut on_remove_edge: impl FnMut(EdgeIndex<Ix>),
) where
    V: Extend<V>,
    Ix: IndexType,
{
    let (p, q) = (quotient.find_mut(p), quotient.find_mut(q));
    if p == q {
        return;
    }
    quotient.union(p, q);
    let keep = quotient.find_mut(p);
    let remove = if keep == p { q } else { p };
    // unify along the source of edges
    for (target, e) in graph
        .edges_directed(remove, Outgoing)
        .filter_map(|e| (e.target() != keep).then(|| (e.target(), e.id())))
        .collect::<Vec<_>>()
    {
        let removed = graph.remove_edge(e).expect("tried to remove missing edge");
        let prev = <&Rewrite0>::try_from(&removed.rewrite)
            .expect("non 0-rewrite passed to collapse unify");
        on_remove_edge(e);
        if !graph
            .edges_connecting(keep, target)
            .map(|existing| {
                <&Rewrite0>::try_from(&existing.weight().rewrite)
                    .expect("non 0-rewrite passed to collapse unify")
                    .label()
            })
            .contains(&prev.label())
        {
            graph.add_edge(keep, target, removed);
        }
    }
    // unify along the target of edges
    for (source, e) in graph
        .edges_directed(remove, Incoming)
        .filter_map(|e| (e.source() != keep).then(|| (e.source(), e.id())))
        .collect::<Vec<_>>()
    {
        let removed = graph.remove_edge(e).expect("tried to remove missing edge");
        let prev = <&Rewrite0>::try_from(&removed.rewrite)
            .expect("non 0-rewrite passed to collapse unify");
        on_remove_edge(e);
        if !graph
            .edges_connecting(source, keep)
            .map(|existing| {
                <&Rewrite0>::try_from(&existing.weight().rewrite)
                    .expect("non 0-rewrite passed to collapse unify")
                    .label()
            })
            .contains(&prev.label())
        {
            graph.add_edge(source, keep, removed);
        }
    }

    let removed = graph
        .remove_node(remove)
        .expect("tried to remove missing node");
    on_remove_node(remove);
    if let Some(k) = graph.node_weight_mut(keep) {
        k.extend(std::iter::once(removed));
    }
}

type Set<T> = OneMany<T, BTreeSet<T>>;

pub(crate) trait Collapsible<V, E, Ix> {
    fn collapse(&self) -> (StableScaffold<Set<V>, E, Ix>, UnionFind<NodeIndex<Ix>>);
}

impl<V, E, Ix> Collapsible<V, E, Ix> for Scaffold<V, E, Ix>
where
    V: Clone + Ord + Cartesian<Height>,
    E: Clone,
    Ix: IndexType,
{
    fn collapse(&self) -> (StableScaffold<Set<V>, E, Ix>, UnionFind<NodeIndex<Ix>>) {
        let mut stable = StableScaffold::from(self.map(
            |_, n| ScaffoldNode::new(Set::One(n.key.clone()), n.diagram.clone()),
            |_, e| e.clone(),
        ));
        let union_find = collapse_stable(&mut stable);
        (stable, union_find)
    }
}

/// Given a **stable** `graph` of 0-diagrams and 0-rewrites, reduce the graph along the
/// *collapsibility* relation, and return the equivalence class on node indices of the induced
/// relation as a [`UnionFind`]. An edge is collapsible exactly when:
/// 1. it is an identity 0-rewrite;
/// 2. all composable triangles formed with this identity 0-rewrite agree label-wise in the other
///    two components.
///
/// # Panics
///
/// Panics if `graph` edges are not 0-rewrites.
pub(crate) fn collapse_stable<V, E, Ix>(
    graph: &mut StableScaffold<V, E, Ix>,
) -> UnionFind<NodeIndex<Ix>>
where
    V: Cartesian<Height> + Extend<V>,
    Ix: IndexType,
{
    // invariant: #nodes of graph = #equivalence classes of union_find
    let mut union_find = UnionFind::new(graph.node_count());
    // tree tracks which edges descended from other edges by graph explosion
    // collapse subproblems need to be solved in topological order, with the root being the final one
    declare_idx! { struct TreeIx = DefaultIx; }
    let tree = {
        let mut tree: DiGraph<_, _, TreeIx> = Default::default();
        let root = tree.add_node((None, OnceCell::new()));
        for (ix, ScaffoldNode { key, .. }) in graph.node_references() {
            let mut cur = root;
            for &c in key.coordinate() {
                if let Some(existing) = tree
                    .neighbors_directed(cur, Incoming)
                    .find(|n| tree[*n].0 == Some(c))
                {
                    cur = existing;
                } else {
                    let next = tree.add_node((Some(c), OnceCell::new()));
                    tree.add_edge(next, cur, ());
                    cur = next;
                }
            }
            tree[cur]
                .1
                .set(vec![ix])
                .expect("failed to initialise collapse subproblem tree");
        }
        tree
    };
    for n in Topo::new(&tree).iter(&tree) {
        // collapse subproblem
        let mut children = tree.neighbors_directed(n, Incoming).detach();
        let mut nodes = vec![];
        while let Some(child) = children.next_node(&tree) {
            nodes.extend_from_slice(tree[child].1.get().unwrap());
        }
        if nodes.is_empty() {
            // n is a leaf
            continue;
        }
        let mut quotient: Vec<_> = Default::default();
        // find collapsible edges wrt nodes
        for e in graph.edge_references().filter(|e| {
            // e is contained within nodes
            nodes.contains(&e.source()) && nodes.contains(&e.target())
            // e is an identity rewrite
            && <&Rewrite0>::try_from(&e.weight().rewrite).unwrap().0.as_ref().map_or(true, |(s, t, _)| s.generator == t.generator)
            // check triangles within nodes which might refute collapsibility of e
            && graph.edges_directed(e.source(), Incoming).all(|p| {
                if let Some(c) = graph.find_edge(p.source(), e.target()) {
                    <&Rewrite0>::try_from(&p.weight().rewrite).unwrap().label() == <&Rewrite0>::try_from(&graph.edge_weight(c).unwrap().rewrite).unwrap().label()
                } else {
                    true
                }
            })
            && graph.edges_directed(e.target(), Outgoing).all(|n| {
                if let Some(c) = graph.find_edge(e.source(), n.target()) {
                    <&Rewrite0>::try_from(&n.weight().rewrite).unwrap().label() == <&Rewrite0>::try_from(&graph.edge_weight(c).unwrap().rewrite).unwrap().label()
                } else {
                    true
                }
            })
        }) {
            // e is collapsible
            quotient.push((e.source(), e.target()));
        }

        for (s, t) in quotient {
            unify(
                graph,
                s,
                t,
                &mut union_find,
                |rn| {
                    nodes.retain(|&n| n != rn);
                },
                |_re| (),
            );
        }
        tree[n]
            .1
            .set(nodes)
            .expect("failed to propagate collapse subproblem");
    }
    // check the tree of collapse subproblems has been completed
    debug_assert!(tree[NodeIndex::new(0)].1.get().is_some());
    union_find
}

impl Diagram {
    pub(crate) fn fully_explode<G>(self) -> G
    where
        G: Default
            + Build
            + ScaffoldGraph<EdgeKey = ()>
            + EdgeCount
            + Index<G::NodeId, Output = G::NodeWeight>,
        for<'a> &'a G: GraphBase<NodeId = G::NodeId, EdgeId = G::EdgeId>
            + IntoNodeReferences<NodeRef = (G::NodeId, &'a G::NodeWeight)>
            + IntoEdgeReferences<EdgeWeight = G::EdgeWeight>,
        G::NodeKey: Clone + Default + IntoIterator<Item = Height> + FromIterator<Height>,
        G::NodeId: Idx,
        G::EdgeId: Idx,
    {
        // Construct the fully exploded scaffold of the diagram.
        let mut scaffold: G = Default::default();
        let dimension = self.dimension();
        scaffold.add_node(self.into());
        for _ in 0..dimension {
            scaffold = scaffold
                .explode_simple(
                    |_, key, si| match si {
                        SliceIndex::Boundary(_) => None,
                        SliceIndex::Interior(h) => Some(
                            Clone::clone(key)
                                .into_iter()
                                .chain(std::iter::once(h))
                                .collect(),
                        ),
                    },
                    |_, _, _| Some(()),
                    |_, _, _| Some(()),
                )
                .unwrap();
        }
        scaffold
    }

    pub(crate) fn label_identifications(self) -> FastHashMap<Coord, Rc<BTreeSet<Coord>>> {
        let (stable, union_find) = self.fully_explode::<Scaffold<Coord>>().collapse();
        union_find
            .into_labeling()
            .into_iter()
            .flat_map(|ix| {
                match stable[ix].key.clone() {
                    OneMany::One(c) => vec![(c.clone(), Rc::new(std::iter::once(c).collect()))],
                    OneMany::Many(cs) => {
                        let shared = Rc::new(cs.clone());
                        cs.into_iter().map(|c| (c, shared.clone())).collect()
                    }
                }
                .into_iter()
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use petgraph::visit::{EdgeRef, IntoEdgeReferences};

    use super::Collapsible;
    use crate::{examples, label::Coord, scaffold::Scaffold, Diagram};

    #[test]
    fn braid_weak_identity() {
        let (_sig, braid) = examples::crossing();
        let weak: Diagram = Diagram::from(braid).weak_identity().into();
        // for each pair of nodes, assert that there is at most one edge (label) between them;
        // otherwise, there is an inconsistency
        let (exploded, _) = weak.fully_explode::<Scaffold<Coord>>().collapse();
        for e in exploded.edge_references() {
            assert_eq!(
                exploded
                    .edges_connecting(e.source(), e.target())
                    .collect::<Vec<_>>(),
                vec![e]
            );
            assert_eq!(exploded.edges_connecting(e.target(), e.source()).count(), 0);
        }
    }
}
