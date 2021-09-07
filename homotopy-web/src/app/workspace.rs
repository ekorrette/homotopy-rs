use std::convert::{Into, TryInto};

use homotopy_core::{
    attach::BoundaryPath,
    common::{Boundary, Height, SliceIndex},
    Diagram, DiagramN,
};
use path_control::PathControl;
use slice_control::SliceControl;
use view_control::ViewControl;
use yew::prelude::*;

use crate::{
    app::diagram2d::{Diagram0D, Diagram1D, Diagram2D, Highlight2D, HighlightKind},
    components::panzoom::PanZoomComponent,
    model::proof::{homotopy::Homotopy, Action, Signature, Workspace},
};

mod path_control;
mod slice_control;
mod view_control;

// TODO: Workspace rerendering when panzoom is changed needs to be smoother.

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
    pub workspace: Workspace,
    pub dispatch: Callback<Action>,
    pub signature: Signature,
}

pub enum Message {}

pub struct WorkspaceView {
    props: Props,
    on_select: Callback<Vec<Vec<SliceIndex>>>,
    on_homotopy: Callback<Homotopy>,
    diagram_ref: NodeRef,
}

impl Component for WorkspaceView {
    type Message = Message;
    type Properties = Props;

    fn create(props: Self::Properties, _link: ComponentLink<Self>) -> Self {
        let on_select = props.dispatch.reform(Action::SelectPoints);
        let on_homotopy = props.dispatch.reform(Action::Homotopy);
        Self {
            props,
            on_select,
            on_homotopy,
            diagram_ref: Default::default(),
        }
    }

    fn update(&mut self, _: Self::Message) -> ShouldRender {
        false
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        props != self.props && {
            self.props = props;
            true
        }
    }

    fn view(&self) -> Html {
        let slice_buttons = match self.visible_diagram() {
            Diagram::Diagram0(_) => Default::default(),
            Diagram::DiagramN(d) => html! {
                <SliceControl
                    number_slices={d.size()}
                    descend_slice={self.props.dispatch.reform(Action::DescendSlice)}
                    diagram_ref={self.diagram_ref.clone()}
                    on_hover={self.props.dispatch.reform(Action::HighlightSlice)}
                />
            },
        };

        html! {
            <div class="workspace">
                <PanZoomComponent on_scroll={self.props.dispatch.reform(Action::SwitchSlice)}>
                    {self.view_diagram()}
                </PanZoomComponent>
                {slice_buttons}
                <div class="workspace__toolbar">
                    <PathControl
                        path={self.props.workspace.path.clone()}
                        ascend_slice={self.props.dispatch.reform(Action::AscendSlice)}
                        dimension={self.props.workspace.diagram.dimension()}
                    />
                    <ViewControl />
                </div>
            </div>
        }
    }
}

impl WorkspaceView {
    fn visible_diagram(&self) -> Diagram {
        // TODO: This should not be recomputed every view
        self.props.workspace.visible_diagram()
    }

    fn view_diagram(&self) -> Html {
        match self.visible_diagram() {
            Diagram::Diagram0(generator) => {
                html! {
                    <Diagram0D
                        diagram={generator}
                        ref={self.diagram_ref.clone()}
                    />
                }
            }
            Diagram::DiagramN(diagram) if diagram.dimension() == 1 => {
                html! {
                    <Diagram1D
                        diagram={diagram}
                        on_select={self.on_select.clone()}
                        ref={self.diagram_ref.clone()}
                    />
                }
            }
            Diagram::DiagramN(diagram) => {
                let highlight = highlight_attachment(&self.props.workspace, &self.props.signature)
                    .or_else(|| highlight_slice(&self.props.workspace));

                html! {
                    <Diagram2D
                        diagram={diagram}
                        id="workspace__diagram"
                        on_select={self.on_select.clone()}
                        on_homotopy={self.on_homotopy.clone()}
                        highlight={highlight}
                        ref={self.diagram_ref.clone()}
                    />
                }
            }
        }
    }
}

fn highlight_attachment(workspace: &Workspace, signature: &Signature) -> Option<Highlight2D> {
    use Height::Regular;

    let attach_option = workspace.attachment_highlight.as_ref()?;
    let needle: DiagramN = signature
        .generator_info(attach_option.generator)?
        .diagram
        .clone()
        .try_into()
        .unwrap();

    let mut boundary_path = attach_option.boundary_path.clone();
    let mut embedding = attach_option.embedding.clone();

    if let Some(BoundaryPath(boundary, depth)) = boundary_path {
        if depth >= workspace.path.len() {
            boundary_path = Some(BoundaryPath(boundary, depth - workspace.path.len()));
        } else {
            boundary_path = None;
            embedding = embedding.skip(workspace.path.len() - depth - 1);
        }
    } else {
        embedding = embedding.skip(workspace.path.len());
    }

    match boundary_path {
        None => {
            // Note: An empty boundary path implies that `needle` is one dimension
            // higher than the currently displayed diagram. Since this function
            // computes highlights for 2d diagrams, the `needle` diagram is at
            // least three-dimensional.
            let needle_s: DiagramN = needle.source().try_into().unwrap();
            let needle_st: DiagramN = needle_s.target().try_into().unwrap();

            Some(Highlight2D {
                from: [Regular(embedding[1]).into(), Regular(embedding[0]).into()],
                to: [
                    Regular(embedding[1] + needle_st.size()).into(),
                    Regular(embedding[0] + needle_s.size()).into(),
                ],
                kind: HighlightKind::Attach,
            })
        }
        Some(bp) if bp.depth() == 0 => {
            let slice: DiagramN = needle
                .slice(bp.boundary().flip())
                .unwrap()
                .try_into()
                .unwrap();
            let size = slice.size();
            Some(Highlight2D {
                from: [Regular(embedding[0]).into(), bp.boundary().into()],
                to: [Regular(embedding[0] + size).into(), bp.boundary().into()],
                kind: HighlightKind::Attach,
            })
        }
        Some(bp) => Some(Highlight2D {
            from: [bp.boundary().into(), Boundary::Source.into()],
            to: [bp.boundary().into(), Boundary::Target.into()],
            kind: HighlightKind::Attach,
        }),
    }
}

fn highlight_slice(workspace: &Workspace) -> Option<Highlight2D> {
    let slice = workspace.slice_highlight.as_ref()?;

    Some(Highlight2D {
        from: [Boundary::Source.into(), *slice],
        to: [Boundary::Target.into(), *slice],
        kind: HighlightKind::Slice,
    })
}
