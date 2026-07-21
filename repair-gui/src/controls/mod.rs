use iced::advanced::Widget;
use iced::advanced::graphics::text::Paragraph;
use iced::advanced::text::Span;
use iced::alignment::Horizontal;
use iced::font::{Family, Stretch, Style, Weight};
use iced::widget::text::Rich;
use iced::widget::{Row, button, container, space, text};
use iced::{Color, Element, Font, Length};
use std::time::Duration;

pub enum Status {
    NotSet,
    Running,
    Success,
    Failure,
}

enum TextBuilderElement<Message> {
    Text(String),
    BoldText(String),
    TypewriterText(String),
    Quad,
    LinkText(String, Message),
    XOutOf { current: i64, total: i64 },
    Time(Duration),
    Status(Status),
}

pub struct TextBuilder<Message> {
    elements: Vec<(TextBuilderElement<Message>, Length)>,
    font_size_factor: f32,
    wrap: bool,
}

impl<Message: Clone> TextBuilder<Message> {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            font_size_factor: 1.0,
            wrap: true,
        }
    }
    pub fn single_line() -> Self {
        Self {
            elements: Vec::new(),
            font_size_factor: 1.0,
            wrap: false,
        }
    }

    pub fn with_text_size(mut self, factor: f32) -> Self {
        self.font_size_factor = factor;
        self
    }

    pub fn with_text<S: Into<String>>(mut self, text: S) -> Self {
        self.elements
            .push((TextBuilderElement::Text(text.into()), Length::Shrink));
        self
    }

    pub fn with_bold<S: Into<String>>(mut self, text: S) -> Self {
        self.elements
            .push((TextBuilderElement::BoldText(text.into()), Length::Shrink));
        self
    }

    pub fn with_space<S: Into<String>>(mut self) -> Self {
        self.elements
            .push((TextBuilderElement::Text(" ".to_string()), Length::Shrink));
        self
    }
    pub fn with_quad(mut self) -> Self {
        self.elements
            .push((TextBuilderElement::Quad, Length::Shrink));
        self
    }

    pub fn with_typewriter<S: Into<String>>(mut self, text: S) -> Self {
        self.elements.push((
            TextBuilderElement::TypewriterText(text.into()),
            Length::Shrink,
        ));
        self
    }

    pub fn with_link<S: Into<String>>(mut self, text: S, on_click: Message) -> Self {
        self.elements.push((
            TextBuilderElement::LinkText(text.into(), on_click),
            Length::Shrink,
        ));
        self
    }

    pub fn with_x_of_y(mut self, current: i64, total: i64) -> Self {
        self.elements.push((
            TextBuilderElement::XOutOf { current, total },
            Length::Shrink,
        ));
        self
    }

    pub fn with_time(mut self, duration: Duration) -> Self {
        self.elements
            .push((TextBuilderElement::Time(duration), Length::Shrink));
        self
    }

    pub fn with_status(mut self, status: Status) -> Self {
        self.elements
            .push((TextBuilderElement::Status(status), Length::Shrink));
        self
    }

    pub fn that_has_width(mut self, width: Length) -> Self {
        if let Some((last, length)) = self.elements.last_mut() {
            *length = width;
        }
        self
    }

    pub fn build<'a>(self) -> Element<'a, Message>
    where
        Message: 'a,
    {
        let mut elements = Row::new();
        let size = 16.0 * self.font_size_factor;
        let font = Font {
            family: Family::SansSerif,
            weight: Weight::Normal,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };
        let bold_font = Font {
            family: Family::SansSerif,
            weight: Weight::Bold,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };
        let typewriter_font = Font {
            family: Family::Monospace,
            weight: Weight::Normal,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };

        for (entry, length) in self.elements {
            match entry {
                TextBuilderElement::Text(content) => {
                    elements = elements.push(text(content).font(font).size(size));
                }
                TextBuilderElement::BoldText(content) => {
                    elements = elements.push(text(content).font(bold_font).size(size));
                }
                TextBuilderElement::TypewriterText(content) => {
                    elements = elements.push(text(content).font(typewriter_font).size(size));
                }
                TextBuilderElement::LinkText(content, msg) => {
                    let text: Rich<(), _, _, _> = Rich::with_spans(vec![
                        Span::new(format!("{} 🔍", content)).underline(true),
                    ])
                    .font(font)
                    .size(size);
                    elements =
                        elements.push(button(text).on_press(msg).padding(0).style(|_, status| {
                            let brightness = match status {
                                button::Status::Active => 0.0,
                                button::Status::Hovered => 0.1,
                                button::Status::Pressed => 0.2,
                                button::Status::Disabled => {
                                    unreachable!()
                                }
                            };
                            button::Style {
                                background: None,
                                text_color: Color::from_rgb(brightness, brightness, brightness),
                                border: Default::default(),
                                shadow: Default::default(),
                                snap: false,
                            }
                        }));
                }
                TextBuilderElement::XOutOf { current, total } => {
                    let mut components = Row::new();
                    components =
                        components.push(text(current.to_string()).font(typewriter_font).size(size));
                    components = components.push(text("/").font(font).size(size));
                    components =
                        components.push(text(total.to_string()).font(typewriter_font).size(size));
                    elements = elements.push(components);
                }
                TextBuilderElement::Time(duration) => {
                    let mut time_components = Row::new();
                    let seconds = duration.as_secs();
                    let minutes = seconds / 60;
                    let hours = minutes / 60;
                    let days = hours / 24;

                    let seconds = seconds % 60;
                    let minutes = minutes % 60;
                    let hours = hours % 24;

                    if days > 0 {
                        time_components = time_components
                            .push(text(days.to_string()).font(typewriter_font).size(size));
                        time_components = time_components.push(text("d ").font(font).size(size));
                    }
                    if hours > 0 || days > 0 {
                        time_components = time_components
                            .push(text(hours.to_string()).font(typewriter_font).size(size));
                        time_components = time_components.push(text("h ").font(font).size(size));
                    }
                    if minutes > 0 || hours > 0 || days > 0 {
                        time_components = time_components
                            .push(text(minutes.to_string()).font(typewriter_font).size(size));
                        time_components = time_components.push(text("m ").font(font).size(size));
                    }
                    time_components = time_components
                        .push(text(seconds.to_string()).font(typewriter_font).size(size));
                    time_components = time_components.push(text("s ").font(font).size(size));

                    elements = elements.push(time_components);
                }
                TextBuilderElement::Status(status) => {
                    let inner: Element<_> = match status {
                        Status::NotSet => space().into(),
                        Status::Running => text("⏳").size(size).into(),
                        Status::Success => text("✓").size(size).into(),
                        Status::Failure => text("✗").size(size).into(),
                    };
                    elements = elements.push(
                        container(inner)
                            .width(size)
                            .height(size)
                            .align_x(Horizontal::Center),
                    );
                }
                TextBuilderElement::Quad => {
                    elements = elements.push(space().width(size).height(size))
                }
            }
        }
        if self.wrap {
            elements.wrap().into()
        } else {
            elements.into()
        }
    }
}
