use std::fs;
use std::process::exit;

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::futures::{channel::mpsc, SinkExt};
use cosmic::iced::subscription::events_with;
use cosmic::iced::wayland::actions::layer_surface::SctkLayerSurfaceSettings;
use cosmic::iced::wayland::layer_surface::{
    destroy_layer_surface, get_layer_surface, Anchor, KeyboardInteractivity,
};
use cosmic::iced::wayland::{InitialSurface, SurfaceIdWrapper};
use cosmic::iced::widget::{button, column, container, text, text_input};
use cosmic::iced::{self, executor, Application, Command, Length, Subscription};
use cosmic::iced_native::event::wayland::LayerEvent;
use cosmic::iced_native::event::{wayland, PlatformSpecific};
use cosmic::iced_native::layout::Limits;
use cosmic::iced_native::widget::helpers;
use cosmic::iced_native::window::Id as SurfaceId;
use cosmic::iced_style::application;
use cosmic::theme::{Button, Container, Svg};
use cosmic::widget::icon;
use cosmic::{keyboard_nav, settings, Element, Theme};
use freedesktop_desktop_entry::DesktopEntry;
use iced::keyboard::KeyCode;
use iced::wayland::Appearance;
use iced::widget::vertical_space;
use iced::{Alignment, Color};
use once_cell::sync::Lazy;
use pop_launcher::{IconSource, SearchResult};

use crate::config;
use crate::subscriptions::launcher::{launcher, LauncherEvent, LauncherRequest};
use crate::subscriptions::toggle_dbus::{dbus_toggle, LauncherDbusEvent};

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

pub fn run() -> cosmic::iced::Result {
    let mut settings = settings();
    settings.exit_on_close_request = false;
    settings.initial_surface = InitialSurface::None;
    CosmicLauncher::run(settings.into())
}

#[derive(Default, Clone)]
struct CosmicLauncher {
    id_ctr: u64,
    input_value: String,
    selected_item: Option<usize>,
    active_surface: Option<SurfaceId>,
    theme: Theme,
    launcher_items: Vec<SearchResult>,
    tx: Option<mpsc::Sender<LauncherRequest>>,
}

#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    Activate(Option<usize>),
    Hide,
    LauncherEvent(LauncherEvent),
    SentRequest,
    Error(String),
    Layer(LayerEvent),
    Toggle,
    Closed,
    KeyboardNav(keyboard_nav::Message),
}

impl Application for CosmicLauncher {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (CosmicLauncher::default(), Command::none())
    }

    fn title(&self) -> String {
        config::APP_ID.to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::InputChanged(value) => {
                self.input_value = value.clone();
                if let Some(tx) = self.tx.as_ref() {
                    let mut tx = tx.clone();
                    let cmd = async move { tx.send(LauncherRequest::Search(value)).await };

                    return Command::perform(cmd, |res| match res {
                        Ok(_) => Message::SentRequest,
                        Err(err) => Message::Error(err.to_string()),
                    });
                }
            }
            Message::Activate(Some(i)) => {
                if let (Some(tx), Some(item)) = (self.tx.as_ref(), self.launcher_items.get(i)) {
                    let mut tx = tx.clone();
                    let id = item.id;
                    let cmd = async move { tx.send(LauncherRequest::Activate(id)).await };
                    return Command::batch(vec![Command::perform(cmd, |res| match res {
                        Ok(_) => Message::Hide,
                        Err(err) => Message::Error(err.to_string()),
                    })]);
                }
            }
            Message::Activate(None) => {
                if let (Some(tx), Some(item)) = (
                    self.tx.as_ref(),
                    self.launcher_items
                        .get(self.selected_item.unwrap_or_default()),
                ) {
                    let mut tx = tx.clone();
                    let id = item.id;
                    let cmd = async move { tx.send(LauncherRequest::Activate(id)).await };
                    return Command::perform(cmd, |res| match res {
                        Ok(_) => Message::SentRequest,
                        Err(err) => Message::Error(err.to_string()),
                    });
                }
            }
            Message::LauncherEvent(e) => match e {
                LauncherEvent::Started(tx) => {
                    let mut tx_clone = tx.clone();
                    let cmd =
                        async move { tx_clone.send(LauncherRequest::Search("".to_string())).await };
                    self.tx.replace(tx);
                    // TODO send the thing as a command
                    return Command::perform(cmd, |res| match res {
                        Ok(_) => Message::SentRequest,
                        Err(err) => Message::Error(err.to_string()),
                    });
                }
                LauncherEvent::Response(response) => match response {
                    pop_launcher::Response::Close => {
                        exit(0);
                    }
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
                                    Some(cmd) if !cmd.contains("=") => {
                                        tokio::process::Command::new(cmd)
                                    }
                                    _ => return Command::none(),
                                };
                                for arg in exec {
                                    // TODO handle "%" args?
                                    if !arg.starts_with("%") {
                                        cmd.arg(arg);
                                    }
                                }
                                let _ = cmd.spawn();
                                return Command::perform(async {}, |_| Message::Hide);
                            }
                        }
                    }
                    pop_launcher::Response::Update(list) => {
                        self.launcher_items.splice(.., list);
                    }
                    pop_launcher::Response::Fill(s) => {
                        self.input_value = s;
                    }
                },
                LauncherEvent::Error(err) => {
                    log::error!("{}", err);
                }
            },
            Message::SentRequest => {}
            Message::Error(err) => {
                log::error!("{}", err);
            }
            Message::Layer(e) => match e {
                LayerEvent::Focused => {
                    return text_input::focus(INPUT_ID.clone());
                }
                LayerEvent::Unfocused => {
                    if let Some(id) = self.active_surface {
                        return destroy_layer_surface(id);
                    }
                }
                _ => {}
            },
            Message::Closed => {
                self.active_surface.take();
                let mut cmds = Vec::new();
                if let Some(tx) = self.tx.as_ref() {
                    let mut tx = tx.clone();
                    let search_cmd =
                        async move { tx.send(LauncherRequest::Search("".to_string())).await };
                    cmds.push(Command::perform(search_cmd, |res| match res {
                        Ok(_) => Message::SentRequest,
                        Err(err) => Message::Error(err.to_string()),
                    }));
                }
                self.input_value = "".to_string();
                cmds.push(text_input::focus(INPUT_ID.clone()));
                return Command::batch(cmds);
            }
            Message::Toggle => {
                if let Some(id) = self.active_surface {
                    return destroy_layer_surface(id);
                } else {
                    self.id_ctr += 1;
                    let mut cmds = Vec::new();
                    if let Some(tx) = self.tx.as_ref() {
                        let mut tx = tx.clone();
                        let search_cmd =
                            async move { tx.send(LauncherRequest::Search("".to_string())).await };
                        cmds.push(Command::perform(search_cmd, |res| match res {
                            Ok(_) => Message::SentRequest,
                            Err(err) => Message::Error(err.to_string()),
                        }));
                    }
                    self.input_value = "".to_string();
                    let id = SurfaceId::new(self.id_ctr);
                    self.active_surface.replace(id);
                    cmds.push(get_layer_surface(SctkLayerSurfaceSettings {
                        id,
                        keyboard_interactivity: KeyboardInteractivity::Exclusive,
                        anchor: Anchor::empty(),
                        namespace: "launcher".into(),
                        size: None,
                        size_limits: Limits::NONE.min_width(1).min_height(1).max_width(600),
                        ..Default::default()
                    }));
                    cmds.push(text_input::focus(INPUT_ID.clone()));
                    return Command::batch(cmds);
                }
            }
            Message::Hide => {
                if let Some(id) = self.active_surface {
                    return destroy_layer_surface(id);
                }
            }
            Message::KeyboardNav(e) => {
                match e {
                    keyboard_nav::Message::FocusNext => return iced::widget::focus_next(),
                    keyboard_nav::Message::FocusPrevious => return iced::widget::focus_previous(),
                    keyboard_nav::Message::Unfocus => {
                        return {
                            self.input_value.clear();
                            if let Some(tx) = self.tx.as_ref() {
                                let mut tx = tx.clone();
                                let cmd = async move {
                                    tx.send(LauncherRequest::Search("".to_string())).await
                                };
                                return Command::perform(cmd, |res| match res {
                                    Ok(_) => Message::SentRequest,
                                    Err(err) => Message::Error(err.to_string()),
                                });
                            }
                            keyboard_nav::unfocus()
                        }
                    }
                    _ => {}
                };
            }
        }
        Command::none()
    }

    fn view(&self, id: SurfaceIdWrapper) -> Element<Message> {
        if id.inner() == SurfaceId::new(0) {
            // TODO just delete the original surface if possible
            return vertical_space(Length::Units(1)).into();
        }

        let launcher_entry = text_input(
            "Type to search apps or type “?” for more options...",
            &self.input_value,
            Message::InputChanged,
        )
        // .on_submit(Message::Activate(None))
        .padding(8)
        .size(20)
        .id(INPUT_ID.clone());

        let buttons = self
            .launcher_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let name = text(item.name.to_string())
                    .horizontal_alignment(Horizontal::Left)
                    .vertical_alignment(Vertical::Center)
                    .size(12);
                let description = text(if item.description.len() > 40 {
                    format!("{:.50}...", item.description)
                } else {
                    item.description.to_string()
                })
                .horizontal_alignment(Horizontal::Left)
                .vertical_alignment(Vertical::Center)
                .size(16);

                let mut button_content = Vec::new();
                if let Some(source) = item.category_icon.as_ref() {
                    let name = match source {
                        IconSource::Name(name) | IconSource::Mime(name) => name,
                    };
                    button_content.push(
                        icon(name.clone(), 64)
                            .theme("Pop")
                            .width(Length::Units(16))
                            .height(Length::Units(16))
                            .style(Svg::Symbolic)
                            .into(),
                    )
                }

                if let Some(source) = item.icon.as_ref() {
                    let name = match source {
                        IconSource::Name(name) | IconSource::Mime(name) => name,
                    };
                    button_content.push(
                        icon(name.clone(), 64)
                            .theme("Pop")
                            .width(Length::Units(32))
                            .height(Length::Units(32))
                            .into(),
                    )
                }

                button_content.push(column![description, name].into());
                button_content.push(
                    container(
                        text(format!("Ctrl + {}", (i + 1) % 10))
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
                    helpers::row(button_content)
                        .spacing(8)
                        .align_items(Alignment::Center),
                )
                .width(Length::Fill)
                .on_press(Message::Activate(Some(i)))
                .padding([8, 16])
                .style(Button::Text);

                btn.into()
            })
            .collect();

        let content = column![launcher_entry, helpers::column(buttons).spacing(16),]
            .spacing(16)
            .max_width(600);

        container(content)
            .style(Container::Custom(|theme| container::Appearance {
                text_color: Some(theme.cosmic().on_bg_color().into()),
                background: Some(theme.extended_palette().background.base.color.into()),
                border_radius: 16.0,
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            }))
            .padding([24, 32])
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(
            vec![
                keyboard_nav::subscription().map(|e| Message::KeyboardNav(e)),
                dbus_toggle(0).map(|e| match e {
                    (_, LauncherDbusEvent::Toggle) => Message::Toggle,
                }),
                launcher(0).map(|(_, msg)| Message::LauncherEvent(msg)),
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
                        KeyCode::Enter => Some(Message::Activate(None)),
                        _ => None,
                    },
                    _ => None,
                }),
            ]
            .into_iter(),
        )
    }

    fn style(&self) -> <Self::Theme as application::StyleSheet>::Style {
        <Self::Theme as application::StyleSheet>::Style::Custom(|theme| Appearance {
            background_color: Color::from_rgba(0.0, 0.0, 0.0, 0.0),
            text_color: theme.cosmic().on_bg_color().into(),
        })
    }

    fn theme(&self) -> Theme {
        self.theme
    }

    fn close_requested(&self, _id: SurfaceIdWrapper) -> Self::Message {
        Message::Closed
    }
}
