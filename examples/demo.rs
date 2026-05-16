use iced::widget::text;
use iced::widget::{button, column, row};
use iced::{Element, Theme};
use iced_waveform::widget::WaveformWidget;

#[derive(Debug, Clone)]
enum Message {
    Sine,
    Square,
    Sawtooth,
    Noise,
}

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(theme)
        .run()
}

fn theme(_state: &App) -> Theme {
    Theme::Dark
}

struct App {
    series: Vec<f32>,
}


impl App {
    fn new() -> Self {
        let data: Vec<f32> = (0..1_000_000).map(|i| (i as f32 * 0.01).sin()).collect();
        App {
            series: data,
        }
    }

    fn update(&mut self, message: Message) {
        let data = match message {
            Message::Sine => make_sine(),
            Message::Square => make_square(),
            Message::Sawtooth => make_sawtooth(),
            Message::Noise => make_noise(),
        };
        self.series = data;
    }



    fn view(&self) -> Element<'_, Message> {
        let btn = |label: &'static str, msg: Message| -> Element<'_, Message> {
            button(text(label)).on_press(msg).into()
        };
        column![
            row![
                btn("Sine", Message::Sine),
                btn("Square", Message::Square),
                btn("Sawtooth", Message::Sawtooth),
                btn("Noise", Message::Noise),
            ]
            .spacing(8),
            WaveformWidget::from_y(self.series.clone()).unwrap(),
        ]
        .spacing(8)
        .padding(8)
        .into()
    }
}


fn make_sine() -> Vec<f32> {
    (0..1_000_000).map(|i| (i as f32 * 0.01).sin()).collect()
}

fn make_square() -> Vec<f32> {
    (0..1_000_000).map(|i| if (i as f32 * 0.01).sin() > 0.0 { 1.0 } else { -1.0 }).collect()
}

fn make_sawtooth() -> Vec<f32> {
    (0..1_000_000).map(|i| (i as f32 * 0.01) % 2.0 - 1.0).collect()
}

fn make_noise() -> Vec<f32> {
    use std::num::Wrapping;
    let mut seed = Wrapping(42u32);
    (0..1_000_000).map(|_| {
        seed = Wrapping(seed.0.wrapping_mul(1664525).wrapping_add(1013904223));
        (seed.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }).collect()
}
