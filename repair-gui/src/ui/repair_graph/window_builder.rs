use iced::border::Radius;
use iced::font::Weight;
use iced::widget::button::Status;
use iced::widget::{Button, Space, button, column, container, row, space, stack, text};
use iced::{Background, Border, Color, Element, Font, Length, Padding};

#[derive(Clone)]
pub enum WindowMessage<Message: Clone> {
    Internal(InternalWindowMessage),
    ContentMessage(Message),
}

#[derive(Clone)]
pub enum InternalWindowMessage {
    SwitchSection { index: usize },
}

pub struct WindowState {
    collapsed: Vec<bool>,
}

impl WindowState {
    pub fn new() -> Self {
        Self {
            collapsed: Vec::new(),
        }
    }

    pub fn with_expanded_sections(number_sections: usize) -> Self {
        Self {
            collapsed: vec![true; number_sections],
        }
    }

    pub fn update(&mut self, message: InternalWindowMessage) {
        match message {
            InternalWindowMessage::SwitchSection { index } => {
                self.collapsed[index] = !self.collapsed[index];
            }
        }
    }
}

pub struct WindowBuilder<'a, 'b, Message: Clone> {
    accent_color: Color,
    padding: f32,
    corner_radius: f32,
    width: f32,
    elements: Vec<Element<'a, WindowMessage<Message>>>,
    section: Option<Section<'a, Message>>,
    section_index: usize,
    state: &'b WindowState,
    last_element_was_divider: bool,
    last_element_was_section: bool,
}

struct Section<'a, Message: Clone> {
    elements: Vec<Element<'a, WindowMessage<Message>>>,
    kind: SectionKind,
    is_expanded: bool,
}

pub struct SectionKind {
    expanded_override: Option<bool>,
    has_toggle_button: bool,
}

impl SectionKind {
    pub fn togglable() -> Self {
        Self {
            expanded_override: None,
            has_toggle_button: true,
        }
    }

    pub fn forced_open() -> Self {
        Self {
            expanded_override: Some(true),
            has_toggle_button: false,
        }
    }

    pub fn forced_close() -> Self {
        Self {
            expanded_override: Some(false),
            has_toggle_button: false,
        }
    }
}

impl<'a, 'b, Message: Clone + 'a> WindowBuilder<'a, 'b, Message> {
    pub fn new(state: &'b WindowState, accent_color: Color, width: f32) -> Self {
        Self {
            accent_color,
            elements: Vec::new(),
            section: None,
            section_index: 0,
            state,
            padding: 6.0,
            corner_radius: 10.0,
            width,
            last_element_was_divider: true, // We treat the start of the window as a divider
            last_element_was_section: false,
        }
    }

    pub fn add_header(&mut self, header: String) {
        let accent_color = self.accent_color;
        let radius = self.corner_radius;
        let title = container(text!["{header}"].font(Font {
            weight: Weight::Bold,
            ..Default::default()
        }))
        .width(Length::Fill)
        .style(move |_| {
            container::Style::default()
                .background(Background::Color(accent_color))
                .border(Border::default().rounded(Radius {
                    top_left: radius,
                    top_right: radius,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                }))
        })
        .padding(Padding::new(self.padding).bottom(5.0));
        self.elements.push(title.into());
    }

    pub fn add_control(&mut self, control: Element<'a, Message>) {
        if self.last_element_was_section {
            self.add_divider();
        }
        if let Some(section) = &mut self.section {
            if section.is_expanded {
                section
                    .elements
                    .push(container(control.map(WindowMessage::ContentMessage)).into());
            }
        } else {
            self.last_element_was_section = false;
            self.last_element_was_divider = false;
            self.elements.push(
                container(control.map(WindowMessage::ContentMessage))
                    .padding(Padding::default().horizontal(self.padding))
                    .into(),
            );
        }
    }

    pub fn start_section<S: Into<String>>(&mut self, summary: S, kind: SectionKind) {
        if self.section.is_some() {
            panic!("Cannot start a section before ending the previous section");
        }
        let summary = summary.into();
        let padding = space().height(4);

        let is_expanded = kind
            .expanded_override
            .unwrap_or(self.state.collapsed[self.section_index]);

        let elements = if is_expanded {
            vec![padding.into()]
        } else {
            vec![padding.into(), text!["{summary}"].into()]
        };
        self.section = Some(Section {
            elements,
            kind,
            is_expanded,
        });
    }

    pub fn end_section(&mut self) {
        if let Some(mut section) = self.section.take() {
            let padding = space().height(4);
            section.elements.push(padding.into());

            let toggle_button_padding = if section.kind.has_toggle_button {
                23.0
            } else {
                0.0
            };
            let content = column(section.elements).width(Length::Fill).padding(
                Padding::default()
                    .left(self.padding)
                    .right(self.padding + toggle_button_padding),
            );
            if section.kind.has_toggle_button {
                let character = if self.state.collapsed[self.section_index] {
                    "▼"
                } else {
                    "◀"
                };
                let expand_button = button(character)
                    .on_press(WindowMessage::Internal(
                        InternalWindowMessage::SwitchSection {
                            index: self.section_index,
                        },
                    ))
                    .style(|_, hovered| {
                        let brightness = match hovered {
                            Status::Active => 0.1,
                            Status::Hovered => 0.2,
                            Status::Pressed => 0.3,
                            Status::Disabled => unreachable!(),
                        };
                        let text_color = Color::from_rgb(brightness, brightness, brightness);
                        button::Style {
                            text_color,
                            ..Default::default()
                        }
                    });

                let content = stack!(
                    content,
                    row!(space().width(Length::Fill), expand_button)
                        .padding(Padding::default().right(self.padding))
                );
                self.elements.push(content.into());
            } else {
                self.elements.push(content.into());
            }
        } else {
            panic!("Cannot end section without first starting it");
        }
        self.section_index += 1;
        self.last_element_was_section = true;
    }

    pub fn add_divider(&mut self) {
        self.last_element_was_divider = true;
        self.last_element_was_section = false;

        let accent_color = self.accent_color;
        let divider = container(Space::new())
            .width(Length::Fill)
            .height(2)
            .style(move |t| {
                container::Style::default().background(Background::Color(accent_color))
            });
        self.elements.push(divider.into());
    }

    pub fn add_call_to_action(&mut self, text: String, message: Option<Message>) {
        if self.section.is_some() {
            panic!("Cannot add call to action while in a section");
        }

        let accent_color = self.accent_color;
        let radius = self.corner_radius;
        let title: Element<Message> = container(
            Button::new(text!["▶ {text}"].width(Length::Fill).center())
                .width(Length::Fill)
                .on_press_maybe(message)
                .style(move |_, status| {
                    let color = match status {
                        Status::Active => accent_color,
                        Status::Hovered => Color::from_rgb(
                            accent_color.r * 0.9,
                            accent_color.g * 0.9,
                            accent_color.b * 0.9,
                        ),
                        Status::Pressed => Color::from_rgb(
                            accent_color.r * 0.85,
                            accent_color.g * 0.85,
                            accent_color.b * 0.85,
                        ),
                        Status::Disabled => accent_color,
                    };

                    button::Style {
                        background: Some(Background::Color(color)),
                        text_color: Color::BLACK,
                        border: Border::default().rounded(Radius {
                            top_right: 0.0,
                            top_left: 0.0,
                            bottom_right: radius,
                            bottom_left: radius,
                        }),
                        shadow: Default::default(),
                        snap: false,
                    }
                })
                .padding(Padding::new(self.padding).bottom(5.0)),
        )
        .width(Length::Fill)
        .into();
        self.elements.push(title.map(WindowMessage::ContentMessage));
    }
    pub fn finish(self) -> Element<'a, WindowMessage<Message>> {
        let container = container(column(self.elements))
            .style(move |t| container::Style {
                text_color: None,
                background: Some(Background::Color(Color::WHITE)),
                border: Border {
                    color: self.accent_color,
                    width: 2.0,
                    radius: Radius::new(self.corner_radius),
                },
                shadow: Default::default(),
                snap: false,
            })
            .width(self.width)
            .clip(true);
        container.into()
    }
}
