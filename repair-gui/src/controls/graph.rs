use glam::Vec2;
use iced::advanced::graphics::geometry;
use iced::advanced::layout::Limits;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, overlay, renderer, widget};
use iced::widget::canvas::Stroke;
use iced::widget::{Space, canvas, space};
use iced::{Color, Element, Event, Length, Point, Rectangle, Size, Vector, mouse};
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone)]
pub struct WidgetGraphState<Id> {
    id_to_node: HashMap<Id, usize>,
    nodes: Vec<GraphNode>,
    connections: Vec<Connection>,
}

impl<Id: Hash + Eq> WidgetGraphState<Id> {
    pub fn new() -> Self {
        Self {
            id_to_node: HashMap::new(),
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    pub fn node(&self, id: Id) -> Option<&GraphNode> {
        Some(&self.nodes[*self.id_to_node.get(&id)?])
    }

    pub fn add_node(&mut self, id: Id, position: Vec2) {
        let index = self.nodes.len();
        self.nodes.push(GraphNode { position });
        self.id_to_node.insert(id, index);
    }

    pub fn add_connection(&mut self, from: Id, to: Id) {
        let from = self.id_to_node[&from];
        let to = self.id_to_node[&to];
        self.connections.push(Connection { from, to })
    }
}

#[derive(Clone)]
pub struct GraphNode {
    position: Vec2,
}

#[derive(Clone)]
pub struct Connection {
    from: usize,
    to: usize,
}

pub struct WidgetGraph<'a, Id, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    state: &'a WidgetGraphState<Id>,
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    width: Length,
    height: Length,
}

impl<'a, Id: Hash + Eq, Message: 'a, Theme, Renderer> WidgetGraph<'a, Id, Message, Theme, Renderer>
where
    Renderer: geometry::Renderer,
{
    pub fn new(state: &'a WidgetGraphState<Id>) -> Self {
        let mut children = Vec::with_capacity(state.nodes.len());
        for _ in 0..state.nodes.len() {
            children.push(space().into())
        }
        Self {
            state,
            children,
            width: Length::Shrink,
            height: Length::Shrink,
        }
    }

    pub fn add_child(&mut self, id: Id, child: impl Into<Element<'a, Message, Theme, Renderer>>) {
        let index = self.state.id_to_node[&id];
        self.children[index] = child.into();
    }
}

impl<Id: Hash + Eq, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for WidgetGraph<'_, Id, Message, Theme, Renderer>
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
        let mut children = Vec::new();
        let mut max = Vec2::new(0.0, 0.0);
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
            let top_mid_position = graph_node.position;
            let top_left_position = top_mid_position - Vec2::new(node.size().width * 0.5, 0.0);
            let bottom_right = top_left_position + Vec2::new(node.size().width, node.size().height);
            let node = node.move_to((top_left_position.x, top_left_position.y));

            children.push(node);
            max = max.max(bottom_right);
        }
        layout::Node::with_children(Size::new(max.x, max.y), children)
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
            let clipped_viewport = if true { &clipped_viewport } else { viewport };

            let mut frame = canvas::Frame::new(renderer, viewport.size());
            for connection in &self.state.connections {
                let from = layout.child(connection.from);
                let to = layout.child(connection.to);

                let start_x = from.bounds().x + from.bounds().width * 0.5;
                let start_y = from.bounds().y + from.bounds().height;
                let end_x = to.bounds().x + to.bounds().width * 0.5;
                let end_y = to.bounds().y;

                let path =
                    canvas::Path::line(Point::new(start_x, start_y), Point::new(end_x, end_y));

                canvas::Path::new(|builder| {
                    builder.move_to(Point::new(start_x, start_y));
                    builder.move_to(Point::new(end_x, end_y))
                });
                frame.stroke(
                    &path,
                    Stroke::default().with_color(Color::BLACK).with_width(3.0),
                );
            }

            renderer.draw_geometry(frame.into_geometry());

            for (child, (tree, layout)) in self
                .children
                .iter()
                .zip(tree.children.iter().zip(layout.children()))
                .filter(|(_, (_, layout))| layout.bounds().intersects(clipped_viewport))
            {
                child.as_widget().draw(
                    tree,
                    renderer,
                    theme,
                    style,
                    layout,
                    cursor,
                    clipped_viewport,
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
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
            .map(|((child, tree), layout)| {
                child
                    .as_widget()
                    .mouse_interaction(tree, layout, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }
}

impl<'a, Id: Hash + Eq, Message, Theme, Renderer>
    From<WidgetGraph<'a, Id, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: geometry::Renderer + 'a,
{
    fn from(graph: WidgetGraph<'a, Id, Message, Theme, Renderer>) -> Self {
        Self::new(graph)
    }
}
