use iced::advanced::layout::{self, Layout, Limits, Node};
use iced::advanced::overlay::{self, Overlay};
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::renderer;
use iced::advanced::text;
use iced::advanced::text::{LineHeight, Paragraph};
use iced::advanced::Shell;
use iced::alignment;
use iced::border;
use iced::mouse;
use iced::widget::shader::Shader;
use iced::{Element, Length, Pixels, Point, Rectangle, Size, Color, Vector};

use crate::curve::CurveItem;
use crate::pipeline::WaveformProgram;
use crate::state::WaveformState;

const LABEL_FONT_SIZE: f32 = 11.0;
const LABEL_COLOR: Color = Color::from_rgb(0.85, 0.85, 0.85);

pub struct WaveformWidget<Message> {
    shader: Shader<Message, WaveformProgram>,
}

impl<Message> WaveformWidget<Message> {
    pub fn from_y(data: Vec<f32>) -> Result<Self, crate::error::Error> {
        let curve = CurveItem::from_y(data)?;
        Ok(Self {
            shader: Shader::new(WaveformProgram::new(vec![curve]))
                .width(Length::Fill)
                .height(Length::Fill),
        })
    }

    pub fn from_xy(_x: Vec<f32>, y: Vec<f32>) -> Result<Self, crate::error::Error> {
        Self::from_y(y)
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for WaveformWidget<Message>
where
    Message: 'static + Clone,
    Renderer: iced::advanced::Renderer
        + text::Renderer
        + iced_wgpu::primitive::Renderer
        + 'static,
{
    fn tag(&self) -> widget::tree::Tag {
        struct Tag<T>(T);
        widget::tree::Tag::of::<Tag<WaveformState>>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(WaveformState::new())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn size_hint(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
    ) -> Node {
        <Shader<Message, WaveformProgram> as Widget<Message, Theme, Renderer>>::layout(
            &mut self.shader, tree, renderer, limits,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        <Shader<Message, WaveformProgram> as Widget<Message, Theme, Renderer>>::draw(
            &self.shader, tree, renderer, theme, style, layout, cursor, viewport,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        <Shader<Message, WaveformProgram> as Widget<Message, Theme, Renderer>>::update(
            &mut self.shader, tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        _layout: Layout<'a>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        let state = tree.state.downcast_ref::<WaveformState>();
        let bounds = Rectangle::new(Point::ORIGIN, _layout.bounds().size());
        let label_specs = state.axis_labels(bounds);

        let tooltip = match (state.cursor_data, state.last_mouse_pos) {
            (Some((cx, cy)), Some(pos)) => Some(TooltipData {
                text: format!("x={:.3}\ny={:.3}", cx, cy),
                position: pos,
            }),
            _ => None,
        };

        if label_specs.is_empty() && tooltip.is_none() {
            return None;
        }

        // Pre-measure every label here in overlay(), so that draw() always
        // has complete position data even if the runtime calls overlay()
        // multiple times and only draws the last instance.
        let font = _renderer.default_font();
        let font_size = Pixels(LABEL_FONT_SIZE);
        let line_h = LineHeight::default().to_absolute(font_size).0;

        let label_draws: Vec<LabelDraw> = label_specs
            .iter()
            .map(|spec| {
                let paragraph = Renderer::Paragraph::with_text(text::Text {
                    content: spec.text.as_str(),
                    bounds: Size::new(10000.0, 10000.0),
                    size: font_size,
                    line_height: LineHeight::default(),
                    font,
                    align_x: text::Alignment::Left,
                    align_y: alignment::Vertical::Top,
                    shaping: text::Shaping::Auto,
                    wrapping: text::Wrapping::None,
                });

                let min = paragraph.min_bounds();

                let x = match spec.align_x {
                    text::Alignment::Center => spec.position.x - min.width / 2.0,
                    text::Alignment::Right => spec.position.x - min.width,
                    _ => spec.position.x,
                };
                let y = match spec.align_y {
                    alignment::Vertical::Center => spec.position.y - line_h / 2.0,
                    alignment::Vertical::Bottom => spec.position.y - line_h,
                    _ => spec.position.y,
                };

                LabelDraw {
                    text: spec.text.clone(),
                    position: Point::new(x, y),
                }
            })
            .collect();

        Some(overlay::Element::new(Box::new(WaveformOverlay {
            label_draws,
            tooltip,
        })))
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        <Shader<Message, WaveformProgram> as Widget<Message, Theme, Renderer>>::mouse_interaction(
            &self.shader, tree, layout, cursor, viewport, renderer,
        )
    }
}

struct TooltipData {
    text: String,
    position: Point,
}

struct LabelDraw {
    text: String,
    position: Point,
}

struct WaveformOverlay {
    label_draws: Vec<LabelDraw>,
    tooltip: Option<TooltipData>,
}

impl<Message, Theme, Renderer> Overlay<Message, Theme, Renderer> for WaveformOverlay
where
    Renderer: iced::advanced::Renderer + text::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let mut children = Vec::new();
        if let Some(ref tooltip) = self.tooltip {
            children.push(tooltip_layout(renderer, tooltip));
        }

        if children.is_empty() {
            layout::Node::new(bounds)
        } else {
            layout::Node::with_children(bounds, children)
        }
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
    ) {
        let clip = layout.bounds();
        let font = renderer.default_font();
        let font_size = Pixels(LABEL_FONT_SIZE);

        for label in &self.label_draws {
            renderer.fill_text(
                text::Text {
                    content: label.text.clone(),
                    bounds: Size::new(10000.0, 10000.0),
                    size: font_size,
                    line_height: LineHeight::default(),
                    font,
                    align_x: text::Alignment::Left,
                    align_y: alignment::Vertical::Top,
                    shaping: text::Shaping::Auto,
                    wrapping: text::Wrapping::None,
                },
                label.position,
                LABEL_COLOR,
                clip,
            );
        }

        if let Some(ref tooltip) = self.tooltip {
            let child_layout = layout.children().next().unwrap();
            draw_tooltip(renderer, tooltip, child_layout);
        }
    }

    fn update(
        &mut self,
        _event: &iced::Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        _shell: &mut Shell<'_, Message>,
    ) {
    }

    fn mouse_interaction(
        &self,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }
}

fn tooltip_layout<R>(renderer: &R, tooltip: &TooltipData) -> Node
where
    R: text::Renderer,
{
    let padding = 6.0;
    let font = renderer.default_font();
    let text = text::Text {
        content: tooltip.text.as_str(),
        bounds: Size::new(10000.0, 10000.0),
        size: Pixels(12.0),
        line_height: LineHeight::default(),
        font,
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Auto,
        wrapping: text::Wrapping::None,
    };
    let paragraph = <R as text::Renderer>::Paragraph::with_text(text);
    let min = paragraph.min_bounds();
    Node::new(Size::new(
        min.width + padding * 2.0,
        min.height + padding * 2.0,
    ))
    .move_to(Point::new(tooltip.position.x + 15.0, tooltip.position.y + 15.0))
}

fn draw_tooltip<R>(renderer: &mut R, tooltip: &TooltipData, layout: Layout<'_>)
where
    R: text::Renderer,
{
    let bounds = layout.bounds();
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: border::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color::from_rgb(0.3, 0.3, 0.3),
            },
            ..Default::default()
        },
        Color::from_rgba(0.05, 0.05, 0.05, 0.92),
    );
    let font = renderer.default_font();
    renderer.fill_text(
        text::Text {
            content: tooltip.text.clone(),
            bounds: Size::new(bounds.width - 12.0, bounds.height - 12.0),
            size: Pixels(12.0),
            line_height: LineHeight::default(),
            font,
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Auto,
            wrapping: text::Wrapping::None,
        },
        Point::new(bounds.x + 6.0, bounds.y + 6.0),
        Color::from_rgb(0.9, 0.9, 0.9),
        bounds,
    );
}

impl<'a, Message> From<WaveformWidget<Message>> for Element<'a, Message>
where
    Message: 'static + Clone,
{
    fn from(widget: WaveformWidget<Message>) -> Self {
        Element::new(widget)
    }
}
