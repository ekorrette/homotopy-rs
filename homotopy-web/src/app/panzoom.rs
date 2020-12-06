use closure::closure;
use euclid::default::{Point2D, Vector2D};
use web_sys::{DomRect, Element, TouchList};
use yew::prelude::*;
use Default;

type Finger = i32;
type Point = Point2D<f64>;
type Vector = Vector2D<f64>;

#[derive(Debug, Clone)]
pub enum Message {
    TouchUpdate(Vec<(Finger, Point)>),
    TouchMove(Vec<(Finger, Point)>),
    MouseWheel(Point, f64),
    MouseDown(Point),
    MouseMove(Point),
    MouseUp,
}

#[derive(Debug, Clone)]
struct State {
    translate: Vector,
    scale: f64,
    mouse: Option<Point>,
    touches: Vec<(Finger, Point)>,
}

impl Default for State {
    fn default() -> Self {
        State {
            translate: Vector::zero(),
            scale: 1.0,
            mouse: None,
            touches: Vec::new(),
        }
    }
}

impl State {
    fn update(&mut self, msg: Message) -> ShouldRender {
        match msg {
            Message::MouseDown(point) => {
                self.mouse = Some(point);
                false
            }
            Message::MouseUp => {
                self.mouse = None;
                false
            }
            Message::MouseMove(next) => {
                match self.mouse {
                    Some(prev) => {
                        self.translate += next - prev;
                        self.mouse = Some(next);
                        true
                    }
                    None => false
                }
            }
            Message::MouseWheel(point, delta) => {
                let scale = self.scale * if delta < 0.0 { 1.1 } else { 1.0 / 1.1 };
                self.translate = point - (point - self.translate) * (scale / self.scale);
                self.scale = scale;
                true
            }
            Message::TouchMove(mut touches) => {
                touches.sort_by_key(|(finger, _)| *finger);

                if touches.len() != 2 || self.touches.len() != 2 {
                    self.touches = touches;
                    return false;
                }

                let average_next = (touches[0].1.to_vector() + touches[1].1.to_vector()) * 0.5;
                let average_prev = (self.touches[0].1.to_vector() + self.touches[1].1.to_vector()) * 0.5;

                let scale = {
                    let distance_prev = (self.touches[0].1 - self.touches[1].1).length().max(0.01);
                    let distance_next = (touches[0].1 - touches[1].1).length().max(0.01);
                    self.scale * (distance_next / distance_prev)
                };

                self.translate = average_next - (average_prev - self.translate) * (scale / self.scale);
                self.scale = scale;
                self.touches = touches;
                true
            }
            Message::TouchUpdate(mut touches) => {
                touches.sort_by_key(|(finger, _)| *finger);
                self.touches = touches;
                false
            }
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Properties)]
pub struct Props {
    pub children: Children,
    #[prop_or_default]
    pub class: String
}

pub struct PanZoom {
    state: State,
    props: Props,
    node_ref: NodeRef,
    on_mouse_move: Callback<MouseEvent>,
    on_mouse_down: Callback<MouseEvent>,
    on_mouse_up: Callback<MouseEvent>,
    on_wheel: Callback<WheelEvent>,
    on_touch_move: Callback<TouchEvent>,
    on_touch_update: Callback<TouchEvent>,
}

impl Component for PanZoom {
    type Message = Message;

    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let node_ref = NodeRef::default();

        let on_mouse_down = Callback::from(closure!(clone link, |e: MouseEvent| {
            if e.ctrl_key() {
                e.prevent_default();
                let x = e.client_x() as f64;
                let y = e.client_y() as f64;
                link.send_message(Message::MouseDown((x, y).into()));
            }
        }));

        let on_mouse_move = Callback::from(closure!(clone link, |e: MouseEvent| {
            e.prevent_default();
            let x = e.client_x() as f64;
            let y = e.client_y() as f64;
            link.send_message(Message::MouseMove((x, y).into()));
        }));

        let on_mouse_up = Callback::from(closure!(clone link, |e: MouseEvent| {
            e.prevent_default();
            link.send_message(Message::MouseUp);
        }));

        let on_wheel = Callback::from(closure!(clone link, clone node_ref, |e: WheelEvent| {
            if e.ctrl_key() {
                e.prevent_default();
                let rect = bounding_rect(&node_ref);
                let x = e.client_x() as f64 - rect.left();
                let y = e.client_y() as f64 - rect.top();
                let delta = e.delta_y();
                link.send_message(Message::MouseWheel((x, y).into(), delta));
            }
        }));

        let on_touch_move = Callback::from(closure!(clone link, clone node_ref, |e: TouchEvent| {
            e.prevent_default();
            let touches = read_touch_list(&e.touches(), &node_ref).collect();
            link.send_message(Message::TouchMove(touches));
        }));

        let on_touch_update =
            Callback::from(closure!(clone link, clone node_ref, |e: TouchEvent| {
                e.prevent_default();
                let touches = read_touch_list(&e.touches(), &node_ref).collect();
                link.send_message(Message::TouchUpdate(touches));
            }));

        PanZoom {
            state: State::default(),
            props,
            node_ref,
            on_mouse_down,
            on_mouse_move,
            on_mouse_up,
            on_wheel,
            on_touch_move,
            on_touch_update,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        self.state.update(msg)
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props == props {
            false
        } else {
            self.props = props;
            true
        }
    }

    fn view(&self) -> Html {
        html! {
            <div
                class={format!("{}", self.props.class)}
                style={self.style_parent()}
                ref={self.node_ref.clone()}
                onmousemove={self.on_mouse_move.clone()}
                onmousedown={self.on_mouse_down.clone()}
                onmouseup={self.on_mouse_up.clone()}
                onwheel={self.on_wheel.clone()}
                ontouchstart={self.on_touch_update.clone()}
                ontouchmove={self.on_touch_move.clone()}
                ontouchend={self.on_touch_update.clone()}
                ontouchcancel={self.on_touch_update.clone()}
            >
                <div style={self.style_child()}>
                    {self.props.children.clone()}
                </div>
            </div>
        }
    }
}

impl PanZoom {
    #[inline(always)]
    fn style_child(&self) -> String {
        format!(
            r#"
                transform-origin: 0 0;
                transform: translate({x}px, {y}px) scale({s});
                position: absolute;
            "#,
            x = self.state.translate.x,
            y = self.state.translate.y,
            s = self.state.scale
        )
    }

    #[inline(always)]
    fn style_parent(&self) -> &'static str {
        r#"
            overflow: hidden;
            touch-action: none;
            position: relative;
        "#
    }
}

#[inline(always)]
fn bounding_rect(node_ref: &NodeRef) -> DomRect {
    node_ref
        .cast::<Element>()
        .unwrap()
        .get_bounding_client_rect()
}

#[inline(always)]
fn read_touch_list<'a>(
    touch_list: &'a TouchList,
    node_ref: &NodeRef,
) -> impl Iterator<Item = (Finger, Point)> + 'a {
    let rect = bounding_rect(node_ref);
    let (rect_left, rect_top) = (rect.left(), rect.top());

    (0..touch_list.length())
        .flat_map(move |i| touch_list.item(i))
        .map(move |touch| {
            let finger = touch.identifier();
            let x = touch.client_x() as f64 - rect_left;
            let y = touch.client_y() as f64 - rect_top;
            (finger, (x, y).into())
        })
}
