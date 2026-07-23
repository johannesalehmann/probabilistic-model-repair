use glam::Vec2;
use iced::advanced::graphics::geometry;
use iced::advanced::layout::Limits;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, overlay, renderer, widget};
use iced::mouse::Button;
use iced::widget::canvas::Stroke;
use iced::widget::text::LineHeight;
use iced::widget::{canvas, space};
use iced::{Color, Element, Event, Length, Point, Rectangle, Size, Vector, mouse};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone)]
pub enum WidgetGraphAction {
    DragStarted {
        cursor_position: Point,
        element: usize,
    },
    DragTo {
        cursor_position: Point,
    },
    DragEnded {
        cursor_position: Option<Point>,
    },
}

#[derive(Clone)]
pub struct WidgetGraphState<Id> {
    id_to_node: HashMap<Id, usize>,
    nodes: Vec<GraphNode>,
    connections: Vec<Connection>,
    drag: Option<Drag>,
    drag_limit_x: Option<f32>,
}

impl<Id: Hash + Eq> WidgetGraphState<Id> {
    pub fn new() -> Self {
        Self {
            id_to_node: HashMap::new(),
            nodes: Vec::new(),
            connections: Vec::new(),
            drag: None,
            drag_limit_x: None,
        }
    }

    pub fn with_drag_limit_x(mut self, limit: f32) -> Self {
        self.drag_limit_x = Some(limit);
        self
    }

    pub fn node(&self, id: Id) -> Option<&GraphNode> {
        Some(&self.nodes[*self.id_to_node.get(&id)?])
    }

    pub fn add_node(&mut self, id: Id, position: Point, width: f32) {
        let index = self.nodes.len();
        self.nodes.push(GraphNode { position, width });
        self.id_to_node.insert(id, index);
    }

    pub fn add_connection(&mut self, from: Id, to: Id) {
        let from = self.id_to_node[&from];
        let to = self.id_to_node[&to];
        self.connections.push(Connection { from, to })
    }

    pub fn update(&mut self, action: WidgetGraphAction) {
        match action {
            WidgetGraphAction::DragStarted {
                cursor_position,
                element,
            } => {
                self.drag = Some(Drag {
                    element,
                    initial_position: self.nodes[element].position,
                    initial_cursor_position: cursor_position,
                })
            }
            WidgetGraphAction::DragTo { cursor_position } => {
                self.drag_to(cursor_position);
            }
            WidgetGraphAction::DragEnded { cursor_position } => {
                if let Some(cursor_position) = cursor_position {
                    self.drag_to(cursor_position)
                }
                self.drag = None
            }
        }
    }
    fn drag_to(&mut self, cursor_position: Point) {
        match &self.drag {
            Some(drag) => {
                let width = self.nodes[drag.element].width;
                let mut position =
                    drag.initial_position + (cursor_position - drag.initial_cursor_position);
                position.x = position.x.max(width * 0.5);
                position.y = position.y.max(0.0);
                if let Some(limit_x) = self.drag_limit_x {
                    position.x = position.x.min(limit_x - width * 0.5);
                }
                self.nodes[drag.element].position = position
            }
            None => {}
        }
    }
}

#[derive(Clone)]
pub struct GraphNode {
    pub position: Point,
    pub width: f32,
}

#[derive(Clone)]
pub struct Connection {
    from: usize,
    to: usize,
}
#[derive(Clone)]
struct Drag {
    element: usize,
    initial_position: Point,
    initial_cursor_position: Point,
}

pub struct WidgetGraph<
    'a,
    Id,
    MsgFact: Fn(WidgetGraphAction) -> Message,
    Message,
    Theme = iced::Theme,
    Renderer = iced::Renderer,
> {
    state: &'a WidgetGraphState<Id>,
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    width: Length,
    height: Length,
    message_factory: MsgFact,
}

impl<'a, Id: Hash + Eq, MsgFact: Fn(WidgetGraphAction) -> Message, Message: 'a, Theme, Renderer>
    WidgetGraph<'a, Id, MsgFact, Message, Theme, Renderer>
where
    Renderer: geometry::Renderer,
{
    pub fn new(state: &'a WidgetGraphState<Id>, message_factory: MsgFact) -> Self {
        let mut children = Vec::with_capacity(state.nodes.len());
        for _ in 0..state.nodes.len() {
            children.push(space().into())
        }
        Self {
            state,
            children,
            width: Length::Shrink,
            height: Length::Shrink,
            message_factory,
        }
    }

    pub fn add_child(&mut self, id: Id, child: impl Into<Element<'a, Message, Theme, Renderer>>) {
        let index = self.state.id_to_node[&id];
        self.children[index] = child.into();
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<Id: Hash + Eq, MsgFact: Fn(WidgetGraphAction) -> Message, Message, Theme, Renderer>
    Widget<Message, Theme, Renderer> for WidgetGraph<'_, Id, MsgFact, Message, Theme, Renderer>
where
    Renderer: geometry::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // When the graph is compressed, this keeps its nodes centered.
        // It may be more desirable to only do this until elements land off-screen, but that
        // requires two passes for layout.
        let offset_x = if let Length::Fixed(width) = self.width {
            let max_width = limits.max().width;
            if max_width < width {
                (max_width - width) * 0.5
            } else {
                0.0
            }
        } else {
            0.0
        };
        let mut children = Vec::new();
        let mut max: Point = Point::new(0.0, 0.0);
        for (index, (graph_node, child)) in self
            .state
            .nodes
            .iter()
            .zip(self.children.iter_mut())
            .enumerate()
        {
            let node =
                child
                    .as_widget_mut()
                    .layout(&mut tree.children[index], renderer, &Limits::NONE);
            let top_mid_position = graph_node.position + Vector::new(offset_x, 0.0);
            let top_left_position = top_mid_position - Vector::new(node.size().width * 0.5, 0.0);
            let bottom_right =
                top_left_position + Vector::new(node.size().width, node.size().height);
            let node = node.move_to((top_left_position.x, top_left_position.y));

            children.push(node);
            max = Point::new(max.x.max(bottom_right.x), max.y.max(bottom_right.y));
        }
        let width = match self.width {
            Length::Fill => limits.max().width,
            Length::FillPortion(_) => limits.max().width,
            Length::Shrink => max.x,
            Length::Fixed(size) => size.min(limits.max().width),
        };
        let height = match self.height {
            Length::Fill => limits.max().height,
            Length::FillPortion(_) => limits.max().height,
            Length::Shrink => max.y,
            Length::Fixed(size) => size,
        };
        layout::Node::with_children(Size::new(width, height), children)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let Some(clipped_viewport) = layout.bounds().intersection(viewport) {
            let mut frame = canvas::Frame::with_bounds(renderer, clipped_viewport);
            for connection in &self.state.connections {
                let from = layout.child(connection.from);
                let to = layout.child(connection.to);
                let start = from.bounds().position()
                    + Vector::new(
                        from.bounds().size().width * 0.5,
                        from.bounds().size().height,
                    );
                let end = to.bounds().position() + Vector::new(to.bounds().width * 0.5, 0.0);

                let path = canvas::Path::line(start, end);

                frame.stroke(
                    &path,
                    Stroke::default().with_color(Color::BLACK).with_width(1.5),
                );

                let direction = end - start;
                if direction != Vector::ZERO {
                    let normalised_direction = direction
                        * (1.0 / (direction.x * direction.x + direction.y * direction.y).sqrt());
                    let orthogonal = Vector::new(normalised_direction.y, -normalised_direction.x);

                    let tip_length = 5.0;
                    let tip_width = 5.0;

                    let tip_left_path = canvas::Path::line(
                        end - normalised_direction * tip_length + orthogonal * tip_width,
                        end,
                    );
                    frame.stroke(
                        &tip_left_path,
                        Stroke::default().with_color(Color::BLACK).with_width(1.5),
                    );

                    let tip_right_path = canvas::Path::line(
                        end - normalised_direction * tip_length - orthogonal * tip_width,
                        end,
                    );
                    frame.stroke(
                        &tip_right_path,
                        Stroke::default().with_color(Color::BLACK).with_width(1.5),
                    );
                }
            }

            renderer.draw_geometry(frame.into_geometry());

            for (child, (tree, layout)) in self
                .children
                .iter()
                .zip(tree.children.iter().zip(layout.children()))
                .filter(|(_, (_, layout))| layout.bounds().intersects(&clipped_viewport))
            {
                child.as_widget().draw(
                    tree,
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    &clipped_viewport,
                )
            }
        }
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            self.children
                .iter_mut()
                .zip(&mut tree.children)
                .zip(layout.children())
                .for_each(|((child, state), layout)| {
                    child
                        .as_widget_mut()
                        .operate(state, layout, renderer, operation);
                });
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, tree), layout) in self
            .children
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            child.as_widget_mut().update(
                tree, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
        if !shell.is_event_captured() {
            if let Event::Mouse(mouse) = event {
                match mouse {
                    mouse::Event::CursorEntered => {}
                    mouse::Event::CursorLeft => {
                        if self.state.drag.is_some() {
                            shell.publish((self.message_factory)(WidgetGraphAction::DragEnded {
                                cursor_position: cursor.position(),
                            }));
                            shell.capture_event();
                        }
                    }
                    mouse::Event::CursorMoved { position } => {
                        if self.state.drag.is_some() {
                            shell.publish((self.message_factory)(WidgetGraphAction::DragTo {
                                cursor_position: *position,
                            }));
                            shell.capture_event();
                        }
                    }
                    mouse::Event::ButtonPressed(button) => {
                        if *button == Button::Left {
                            for (index, child) in layout.children().enumerate() {
                                if cursor.is_over(child.bounds()) {
                                    shell.publish((self.message_factory)(
                                        WidgetGraphAction::DragStarted {
                                            cursor_position: cursor.position().unwrap_or_else(
                                                || self.state.nodes[index].position,
                                            ),
                                            element: index,
                                        },
                                    ));
                                    shell.capture_event();
                                    break;
                                }
                            }
                        }
                    }
                    mouse::Event::ButtonReleased(button) => {
                        if self.state.drag.is_some() {
                            if *button == Button::Left {
                                shell.publish((self.message_factory)(
                                    WidgetGraphAction::DragEnded {
                                        cursor_position: cursor.position(),
                                    },
                                ));
                                shell.capture_event();
                            }
                        }
                    }
                    mouse::Event::WheelScrolled { .. } => {}
                }
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            let child_interaction = self
                .children
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
                .map(|((child, tree), layout)| {
                    child
                        .as_widget()
                        .mouse_interaction(tree, layout, cursor, viewport, renderer)
                })
                .max();
            match child_interaction {
                Some(mouse::Interaction::None) | None => {
                    for child in layout.children() {
                        if cursor.is_over(child.bounds()) {
                            return mouse::Interaction::Grab;
                        }
                    }
                    Default::default()
                }
                Some(interaction) => interaction,
            }
        } else {
            Default::default()
        }
    }
}

impl<'a, Id: Hash + Eq, MsgFact: Fn(WidgetGraphAction) -> Message + 'a, Message, Theme, Renderer>
    From<WidgetGraph<'a, Id, MsgFact, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: geometry::Renderer + 'a,
{
    fn from(graph: WidgetGraph<'a, Id, MsgFact, Message, Theme, Renderer>) -> Self {
        Self::new(graph)
    }
}
