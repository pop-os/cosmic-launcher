use std::fs;

use crate::config;
use crate::subscriptions::launcher;
use crate::subscriptions::toggle_dbus::{dbus_toggle, LauncherDbusEvent};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::id::Id;
use cosmic::iced::subscription::events_with;
use cosmic::iced::wayland::actions::layer_surface::SctkLayerSurfaceSettings;
use cosmic::iced::wayland::layer_surface::{
    destroy_layer_surface, get_layer_surface, Anchor, KeyboardInteractivity,
};
use cosmic::iced::wayland::InitialSurface;
use cosmic::iced::widget::{button, column, container, text, text_input, Column};
use cosmic::iced::{self, Application, Command, Length, Subscription};
use cosmic::iced_runtime::core::event::wayland::LayerEvent;
use cosmic::iced_runtime::core::event::{wayland, PlatformSpecific};
use cosmic::iced_runtime::core::layout::Limits;
use cosmic::iced_runtime::core::window::Id as SurfaceId;
use cosmic::iced_style::application;
use cosmic::iced_widget::row;
use cosmic::iced_widget::text_input::{Icon, Side};
use cosmic::theme::{self, Button, Container, Svg, TextInput};
use cosmic::widget::{divider, icon};
use cosmic::{keyboard_nav, settings, Element, Theme};
use freedesktop_desktop_entry::DesktopEntry;
use iced::keyboard::KeyCode;
use iced::wayland::Appearance;
use iced::widget::vertical_space;
use iced::{Alignment, Color};
use once_cell::sync::Lazy;
use pop_launcher::{IconSource, SearchResult};
use tokio::sync::mpsc;

static INPUT_ID: Lazy<Id> = Lazy::new(|| Id::new("input_id"));

const WINDOW_ID: SurfaceId = SurfaceId(1);

pub fn run() -> cosmic::iced::Result {
    let mut settings = settings();
    settings.exit_on_close_request = false;
    settings.initial_surface = InitialSurface::None;
    CosmicLauncher::run(settings)
}

#[derive(Default, Clone)]
struct CosmicLauncher {
    input_value: String,
    selected_item: Option<usize>,
    active_surface: bool,
    theme: Theme,
    launcher_items: Vec<SearchResult>,
    tx: Option<mpsc::Sender<launcher::Request>>,
}

#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    Activate(Option<usize>),
    Hide,
    LauncherEvent(launcher::Event),
    Layer(LayerEvent),
    Toggle,
    Closed,
    KeyboardNav(keyboard_nav::Message),
    Ignore,
}

impl CosmicLauncher {
    fn hide(&mut self) -> Command<Message> {
        self.input_value.clear();

        if self.active_surface {
            self.active_surface = false;
            return destroy_layer_surface(WINDOW_ID);
        }

        Command::none()
    }
}

impl Application for CosmicLauncher {
    type Message = Message;
    type Theme = Theme;
    type Executor = cosmic::executor::single::Executor;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (CosmicLauncher::default(), Command::none())
    }

    fn title(&self) -> String {
        config::APP_ID.to_string()
    }

    #[allow(clippy::too_many_lines)]
    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::InputChanged(value) => {
                self.input_value = value.clone();
                if let Some(tx) = &self.tx {
                    let _res = tx.blocking_send(launcher::Request::Search(value));
                }
            }
            Message::Activate(Some(i)) => {
                if let (Some(tx), Some(item)) = (&self.tx, self.launcher_items.get(i)) {
                    let _res = tx.blocking_send(launcher::Request::Activate(item.id));
                }
            }
            Message::Activate(None) => {
                if let (Some(tx), Some(item)) = (
                    &self.tx,
                    self.launcher_items
                        .get(self.selected_item.unwrap_or_default()),
                ) {
                    let _res = tx.blocking_send(launcher::Request::Activate(item.id));
                }
            }
            Message::LauncherEvent(e) => match e {
                launcher::Event::Started(tx) => {
                    _ = tx.blocking_send(launcher::Request::Search(String::new()));
                    self.tx.replace(tx);
                }
                launcher::Event::Response(response) => match response {
                    pop_launcher::Response::Close => return self.hide(),
                    pop_launcher::Response::Context { .. } => {
                        // TODO ASHLEY
                    }
                    pop_launcher::Response::DesktopEntry {
                        path,
                        gpu_preference: _,
                    } => {
                        if let Ok(bytes) = fs::read_to_string(&path) {
                            if let Ok(entry) = DesktopEntry::decode(&path, &bytes) {
                                let mut exec = match entry.exec() {
                                    Some(exec_str) => shlex::Shlex::new(exec_str),
                                    _ => return Command::none(),
                                };
                                let mut cmd = match exec.next() {
                                    Some(cmd) if !cmd.contains('=') => {
                                        std::process::Command::new(cmd)
                                    }
                                    _ => return Command::none(),
                                };
                                for arg in exec {
                                    // TODO handle "%" args?
                                    if !arg.starts_with('%') {
                                        cmd.arg(arg);
                                    }
                                }
                                crate::process::spawn(cmd);
                                return self.hide();
                            }
                        }
                    }
                    pop_launcher::Response::Update(mut list) => {
                        list.sort_by(|a, b| {
                            let a = i32::from(a.window.is_none());
                            let b = i32::from(b.window.is_none());
                            a.cmp(&b)
                        });
                        self.launcher_items.splice(.., list);
                    }
                    pop_launcher::Response::Fill(s) => {
                        self.input_value = s;
                        return text_input::focus(INPUT_ID.clone());
                    }
                },
            },
            Message::Layer(e) => match e {
                LayerEvent::Focused => {
                    return text_input::focus(INPUT_ID.clone());
                }
                LayerEvent::Unfocused => {
                    if self.active_surface {
                        self.active_surface = false;
                        return destroy_layer_surface(WINDOW_ID);
                    }
                }
                LayerEvent::Done => {}
            },
            Message::Closed => {
                self.active_surface = false;
                self.input_value = String::new();
                return text_input::focus(INPUT_ID.clone());
            }
            Message::Toggle => {
                if self.active_surface {
                    self.active_surface = false;
                    return destroy_layer_surface(WINDOW_ID);
                }

                if let Some(tx) = &self.tx {
                    let _res = tx.blocking_send(launcher::Request::Search(String::new()));
                } else {
                    log::info!("NOT FOUND");
                }

                self.input_value = String::new();
                self.active_surface = true;

                return Command::batch(vec![
                    get_layer_surface(SctkLayerSurfaceSettings {
                        id: WINDOW_ID,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        anchor: Anchor::TOP,
                        namespace: "launcher".into(),
                        size: None,
                        margin: iced::wayland::actions::layer_surface::IcedMargin {
                            top: 16,
                            ..Default::default()
                        },
                        size_limits: Limits::NONE.min_width(1.0).min_height(1.0).max_width(600.0),
                        ..Default::default()
                    }),
                    text_input::focus(INPUT_ID.clone()),
                ]);
            }
            Message::Hide => return self.hide(),
            Message::KeyboardNav(e) => {
                match e {
                    keyboard_nav::Message::FocusNext => return iced::widget::focus_next(),
                    keyboard_nav::Message::FocusPrevious => return iced::widget::focus_previous(),
                    keyboard_nav::Message::Unfocus => {
                        return {
                            self.input_value.clear();
                            if let Some(tx) = &self.tx {
                                let _res =
                                    tx.blocking_send(launcher::Request::Search(String::new()));
                            }
                            keyboard_nav::unfocus()
                        }
                    }
                    _ => {}
                };
            }
            Message::Ignore => {}
        }
        Command::none()
    }

    #[allow(clippy::too_many_lines)]
    fn view(&self, id: SurfaceId) -> Element<Message> {
        if id == SurfaceId(0) {
            // TODO just delete the original surface if possible
            return vertical_space(Length::Fixed(1.0)).into();
        }

        let launcher_entry = text_input(
            "Type to search apps or type “?” for more options...",
            &self.input_value,
        )
        .on_input(Message::InputChanged)
        .on_paste(Message::InputChanged)
        .on_submit(Message::Activate(None))
        .size(14)
        .style(TextInput::Search)
        .padding([8, 24])
        .icon(Icon {
            font: iced::Font::default(),
            code_point: '🔍',
            size: Some(12.0),
            spacing: 12.0,
            side: Side::Left,
        })
        .id(INPUT_ID.clone());

        let buttons: Vec<_> = self
            .launcher_items
            .iter()
            .enumerate()
            .flat_map(|(i, item)| {
                let (name, desc) = if item.window.is_some() {
                    (&item.description, &item.name)
                } else {
                    (&item.name, &item.description)
                };

                let name = Column::with_children(
                    name.lines()
                        .map(|line| {
                            text(if line.len() > 45 {
                                format!("{line:.45}...")
                            } else {
                                line.to_string()
                            })
                            .horizontal_alignment(Horizontal::Left)
                            .vertical_alignment(Vertical::Center)
                            .size(14)
                            .into()
                        })
                        .collect(),
                );
                let desc = Column::with_children(
                    desc.lines()
                        .map(|line| {
                            text(if line.len() > 60 {
                                format!("{line:.60}")
                            } else {
                                line.to_string()
                            })
                            .horizontal_alignment(Horizontal::Left)
                            .vertical_alignment(Vertical::Center)
                            .size(10)
                            .style(theme::Text::Accent)
                            .into()
                        })
                        .collect(),
                );

                let mut button_content = Vec::new();
                if let Some(source) = item.category_icon.as_ref() {
                    let name = match source {
                        IconSource::Name(name) | IconSource::Mime(name) => name,
                    };
                    button_content.push(
                        icon(name.clone(), 64)
                            .theme("Pop")
                            .width(Length::Fixed(16.0))
                            .height(Length::Fixed(16.0))
                            .style(Svg::Symbolic)
                            .into(),
                    );
                }

                if let Some(source) = item.icon.as_ref() {
                    let name = match source {
                        IconSource::Name(name) | IconSource::Mime(name) => name,
                    };
                    button_content.push(
                        icon(name.clone(), 64)
                            .theme("Pop")
                            .width(Length::Fixed(32.0))
                            .height(Length::Fixed(32.0))
                            .into(),
                    );
                }

                button_content.push(column![name, desc].into());
                button_content.push(
                    container(
                        text(format!("Ctrl + {}", (i + 1) % 10))
                            .size(14)
                            .vertical_alignment(Vertical::Center)
                            .horizontal_alignment(Horizontal::Right),
                    )
                    .width(Length::Fill)
                    .center_y()
                    .align_y(Vertical::Center)
                    .align_x(Horizontal::Right)
                    .padding([8, 16])
                    .into(),
                );

                let btn = button(
                    row(button_content)
                        .spacing(8)
                        .align_items(Alignment::Center),
                )
                .width(Length::Fill)
                .on_press(Message::Activate(Some(i)))
                .padding([8, 16])
                .style(Button::Custom {
                    active: Box::new(|theme| {
                        let text = button::StyleSheet::active(theme, &Button::Text);
                        button::Appearance {
                            border_radius: 8.0,
                            ..text
                        }
                    }),
                    hover: Box::new(|theme| {
                        let text = button::StyleSheet::hovered(theme, &Button::Text);
                        button::Appearance {
                            border_radius: 8.0,
                            ..text
                        }
                    }),
                });
                if i == self.launcher_items.len() - 1 {
                    vec![btn.into()]
                } else {
                    vec![btn.into(), divider::horizontal::light().into()]
                }
            })
            .collect();

        let mut content = column![launcher_entry].max_width(600).spacing(16);

        if !buttons.is_empty() {
            content = content.push(column(buttons));
        }
        container(content)
            .style(Container::Custom(Box::new(|theme| container::Appearance {
                text_color: Some(theme.cosmic().on_bg_color().into()),
                background: Some(Color::from(theme.cosmic().background.base).into()),
                border_radius: 16.0.into(),
                border_width: 1.0,
                border_color: theme.cosmic().bg_divider().into(),
            })))
            .padding([24, 32])
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(
            vec![
                keyboard_nav::subscription().map(Message::KeyboardNav),
                dbus_toggle(0).map(|e| match e {
                    Some((_, LauncherDbusEvent::Toggle)) => Message::Toggle,
                    None => Message::Ignore,
                }),
                launcher::subscription(0).map(Message::LauncherEvent),
                events_with(|e, _status| match e {
                    cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                        wayland::Event::Layer(e, ..),
                    )) => Some(Message::Layer(e)),
                    cosmic::iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
                        key_code,
                        modifiers,
                    }) => match key_code {
                        KeyCode::Escape => Some(Message::Hide),
                        KeyCode::Key1 | KeyCode::Numpad1 if modifiers.control() => {
                            Some(Message::Activate(Some(0)))
                        }
                        KeyCode::Key2 | KeyCode::Numpad2 if modifiers.control() => {
                            Some(Message::Activate(Some(1)))
                        }
                        KeyCode::Key3 | KeyCode::Numpad3 if modifiers.control() => {
                            Some(Message::Activate(Some(2)))
                        }
                        KeyCode::Key4 | KeyCode::Numpad4 if modifiers.control() => {
                            Some(Message::Activate(Some(3)))
                        }
                        KeyCode::Key5 | KeyCode::Numpad5 if modifiers.control() => {
                            Some(Message::Activate(Some(4)))
                        }
                        KeyCode::Key6 | KeyCode::Numpad6 if modifiers.control() => {
                            Some(Message::Activate(Some(5)))
                        }
                        KeyCode::Key7 | KeyCode::Numpad7 if modifiers.control() => {
                            Some(Message::Activate(Some(6)))
                        }
                        KeyCode::Key8 | KeyCode::Numpad7 if modifiers.control() => {
                            Some(Message::Activate(Some(7)))
                        }
                        KeyCode::Key9 | KeyCode::Numpad9 if modifiers.control() => {
                            Some(Message::Activate(Some(8)))
                        }
                        KeyCode::Key0 | KeyCode::Numpad0 if modifiers.control() => {
                            Some(Message::Activate(Some(9)))
                        }
                        KeyCode::Up => {
                            Some(Message::KeyboardNav(keyboard_nav::Message::FocusPrevious))
                        }
                        KeyCode::Down => {
                            Some(Message::KeyboardNav(keyboard_nav::Message::FocusNext))
                        }
                        KeyCode::P | KeyCode::K if modifiers.control() => {
                            Some(Message::KeyboardNav(keyboard_nav::Message::FocusPrevious))
                        }
                        KeyCode::N | KeyCode::J if modifiers.control() => {
                            Some(Message::KeyboardNav(keyboard_nav::Message::FocusNext))
                        }
                        _ => None,
                    },
                    _ => None,
                }),
            ]
            .into_iter(),
        )
    }

    fn style(&self) -> <Self::Theme as application::StyleSheet>::Style {
        <Self::Theme as application::StyleSheet>::Style::Custom(Box::new(|theme| Appearance {
            background_color: Color::from_rgba(0.0, 0.0, 0.0, 0.0),
            text_color: theme.cosmic().on_bg_color().into(),
        }))
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn close_requested(&self, _id: SurfaceId) -> Self::Message {
        Message::Closed
    }
}
