use boundary::BoundaryPreview;
use settings::AppSettings;
use sidebar::Sidebar;
use signature_stylesheet::SignatureStylesheet;
use wasm_bindgen::{closure::Closure, JsCast};
use workspace::WorkspaceView;
use yew::prelude::*;

use self::diagram_gl::GlViewControl;
use crate::{
    components::{
        icon::{Icon, IconSize},
        panzoom::PanZoom,
        settings::Settings,
        toast::{Toast, Toaster, ToasterComponent},
    },
    model,
};

mod attach;
mod boundary;
#[cfg(debug_assertions)]
mod debug;
mod diagram_gl;
mod diagram_svg;
mod keybindings;
mod project;
mod settings;
mod sidebar;
mod signature;
mod signature_stylesheet;
mod workspace;

#[derive(Default, Clone, Debug, PartialEq, Properties)]
pub struct Props {}

pub enum Message {
    Dispatch(model::Action),
}

pub struct App {
    state: model::State,
    panzoom: PanZoom,
    orbit_control: GlViewControl,
    signature_stylesheet: SignatureStylesheet,
    toaster: Toaster,
    _settings: AppSettings,
    before_unload: Option<Closure<dyn FnMut(web_sys::BeforeUnloadEvent)>>,
}

impl Component for App {
    type Message = Message;
    type Properties = Props;

    #[allow(unused_variables)]
    fn create(ctx: &Context<Self>) -> Self {
        let state = model::State::default();
        // Install the signature stylesheet
        let mut signature_stylesheet = SignatureStylesheet::new("generator");
        signature_stylesheet.update(state.with_proof(|p| p.signature().clone()));
        signature_stylesheet.mount();

        let mut app = Self {
            state,
            panzoom: PanZoom::new(),
            orbit_control: GlViewControl::new(),
            signature_stylesheet,
            toaster: Toaster::new(),
            _settings: AppSettings::connect(Callback::noop()),
            before_unload: None,
        };
        app.install_unload_hook();
        app
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Message::Dispatch(action) => {
                if !self.state.with_proof(|proof| action.is_valid(proof)) {
                    return false;
                }

                log::info!("Received action: {:?}", action);

                if let model::Action::Proof(ref action) = action {
                    if self.state.with_proof(|p| p.resets_panzoom(action)) {
                        self.panzoom.reset();
                        self.orbit_control.reset();
                    }
                }

                let performance = web_sys::window().unwrap().performance().unwrap();
                performance.mark("startStateUpdate").unwrap();
                let result = self.state.update(action);
                performance.mark("stopStateUpdate").unwrap();
                performance
                    .measure_with_start_mark_and_end_mark(
                        "stateUpdate",
                        "startStateUpdate",
                        "stopStateUpdate",
                    )
                    .unwrap();
                log::info!(
                    "State update took {}ms.",
                    js_sys::Reflect::get(
                        &performance
                            .get_entries_by_name_with_entry_type("stateUpdate", "measure")
                            .get(0),
                        &wasm_bindgen::JsValue::from_str("duration")
                    )
                    .unwrap()
                    .as_f64()
                    .unwrap()
                );

                performance.clear_marks();
                performance.clear_measures();

                homotopy_core::collect_garbage();

                if let Err(error) = result {
                    self.toaster.toast(Toast::error(format!("{}", error)));
                    log::error!("Error occured: {}", error);
                }

                self.signature_stylesheet
                    .update(self.state.with_proof(|p| p.signature().clone()));

                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        Self::render(ctx, &self.state)
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        self.signature_stylesheet.unmount();
    }
}

impl App {
    fn install_unload_hook(&mut self) {
        let before_unload = Closure::wrap(Box::new(move |event: web_sys::BeforeUnloadEvent| {
            event.set_return_value("Are you sure you want to leave? Unsaved changes will be lost!");
        }) as Box<dyn FnMut(_)>);

        web_sys::window()
            .unwrap()
            .set_onbeforeunload(Some(before_unload.as_ref().unchecked_ref()));

        self.before_unload = Some(before_unload);
    }

    fn render(ctx: &Context<Self>, state: &model::State) -> Html {
        let proof = state.with_proof(Clone::clone);
        let dispatch = ctx.link().callback(Message::Dispatch);
        let signature = proof.signature();

        let workspace = match proof.workspace() {
            Some(workspace) => {
                html! {
                    <WorkspaceView
                        workspace={workspace.clone()}
                        signature={signature.clone()}
                        dispatch={dispatch.reform(model::Action::Proof)}
                    />
                }
            }
            None => {
                // TODO: Show onboarding info if workspace and signature is empty
                html! {
                    <content class="workspace workspace--empty">
                    </content>
                }
            }
        };

        let boundary_preview = match proof.boundary() {
            Some(b) => html! {
                <BoundaryPreview
                    boundary={b.clone()}
                />
            },
            None => Default::default(),
        };

        html! {
            <main class="app">
                <Sidebar
                    dispatch={dispatch}
                    proof={proof}
                />
                <ToasterComponent timeout={3000} />
                <div class="boundary__and__workspace">
                    {boundary_preview}
                    {workspace}
                </div>
                <div id="about" class="modal">
                    <div class="modal-dialog">
                        <a href="#">
                            // Empty div to create an invisible button
                            <div class="modal-close"></div>
                        </a>
                        <div class="modal-content">
                            <header>
                                <h2>{"About"}</h2>
                            </header>
                            <p>
                                <a href="https://ncatlab.org/nlab/show/homotopy.io">{"homotopy.io"}</a>
                                {": the proof assistant for finitely-presented globular n-categories."}
                            </p>
                            <p>{"Written by "}
                                <a href="https://github.com/doctorn">{"Nathan Corbyn"}</a>
                                {", "}
                                <a href="https://github.com/zrho">{"Lukas Heidemann"}</a>
                                {", "}
                                <a href="https://github.com/NickHu">{"Nick Hu"}</a>
                                {", "}
                                <a href="https://github.com/calintat">{"Calin Tataru"}</a>
                                {", and "}
                                <a href="https://github.com/jamievicary">{"Jamie Vicary"}</a>
                                {"."}
                            </p>
                            <h3>{"License"}</h3>
                            <p>{"homotopy.io source code is published under the terms of the BSD 3-Clause License."}</p>
                            <pre>{include_str!("../../LICENSE")}</pre>
                            {"homotopy.io documentation is licensed under a "}
                            <a rel="license" href="http://creativecommons.org/licenses/by/4.0/">
                                {"Creative Commons Attribution 4.0 International License"}
                            </a>{"."}
                            <br />
                            <a rel="license" href="http://creativecommons.org/licenses/by/4.0/">
                                <img alt="Creative Commons License" style="border-width:0" src="by.svg" />
                            </a>
                        </div>
                    </div>
                </div>
                <span class="version">
                    {format!("Version: {}", option_env!("GIT_DESCRIBE").unwrap_or(env!("CARGO_PKG_VERSION")))}
                </span>
            </main>
        }
    }
}
