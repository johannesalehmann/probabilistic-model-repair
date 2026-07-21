use iced::gradient::Linear;
use iced::widget::{container, row, space, stack};
use iced::{Background, Color, Element, Gradient, Length};

pub fn fadeout_box<'a, Msg: Clone + 'a>(
    content: Element<'a, Msg>,
    size: Length,
) -> Element<'a, Msg> {
    fadeout_box_custom(content, size, 20.0)
}
pub fn fadeout_box_custom<'a, Msg: Clone + 'a>(
    content: Element<'a, Msg>,
    size: Length,
    fadeout_length: f32,
) -> Element<'a, Msg> {
    stack![
        container(content).width(size).clip(true),
        row![
            space().width(Length::Fill),
            container(space())
                .width(fadeout_length)
                .height(Length::Fill)
                .style(|t| container::Style::default().background(
                    Background::Gradient(Gradient::Linear(
                        Linear::new(std::f32::consts::FRAC_PI_2)
                            .add_stop(0.0, Color::from_rgba(1.0, 1.0, 1.0, 0.0))
                            .add_stop(1.0, Color::from_rgba(1.0, 1.0, 1.0, 1.0))
                    )) // Background::Color(Color::BLACK)
                ))
        ]
    ]
    .into()
}
