use std::ffi::OsStr;
use std::fs;
use std::process::exit;

use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::futures::{channel::mpsc, SinkExt};
use cosmic::iced::subscription::events_with;
use cosmic::iced::widget::{button, column, container, row, text, text_input};
use cosmic::iced::{executor, Application, Command, Length, Subscription};
use cosmic::iced_native::widget::helpers;
use cosmic::iced_native::window::Id as SurfaceId;
use cosmic::iced_style::{self, application};
use cosmic::theme::{Button, Container, Svg};
use cosmic::{settings, widget, Element, Theme};
use freedesktop_desktop_entry::DesktopEntry;
use iced::keyboard::KeyCode;
use iced::wayland::Appearance;
use iced::widget::{svg, vertical_space, Image};
use iced::{Alignment, Color};
use iced_sctk::application::SurfaceIdWrapper;
use iced_sctk::command::platform_specific::wayland::layer_surface::SctkLayerSurfaceSettings;
use iced_sctk::commands;
use iced_sctk::commands::layer_surface::{Anchor, KeyboardInteractivity, Layer};
use iced_sctk::event::wayland::LayerEvent;
use iced_sctk::event::{wayland, PlatformSpecific};
use iced_sctk::settings::InitialSurface;
use once_cell::sync::Lazy;
use pop_launcher::{IconSource, SearchResult};

use crate::config;
use crate::subscriptions::launcher::{launcher, LauncherEvent, LauncherRequest};
use crate::subscriptions::toggle_dbus::{dbus_toggle, LauncherDbusEvent};

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

pub fn run() -> cosmic::iced::Result {
    let mut settings = settings();
    settings.exit_on_close_request = false;
    settings.initial_surface = InitialSurface::LayerSurface(SctkLayerSurfaceSettings {
        keyboard_interactivity: KeyboardInteractivity::None,
        namespace: "ignore".into(),
        size: (Some(1), Some(1)),
        layer: Layer::Background,
        ..Default::default()
    });
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
    SelectPrev,
    SelectNext,
    Clear,
    LauncherEvent(LauncherEvent),
    SentRequest,
    Error(String),
    Layer(LayerEvent),
    Toggle,
    Closed,
}

impl Application for CosmicLauncher {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            CosmicLauncher::default(),
            commands::layer_surface::destroy_layer_surface(SurfaceId::new(0)),
        )
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
            Message::Clear => {
                self.input_value.clear();
                if let Some(tx) = self.tx.as_ref() {
                    let mut tx = tx.clone();
                    let cmd = async move { tx.send(LauncherRequest::Search("".to_string())).await };
                    return Command::perform(cmd, |res| match res {
                        Ok(_) => Message::SentRequest,
                        Err(err) => Message::Error(err.to_string()),
                    });
                }
            }
            Message::SentRequest => {}
            Message::Error(err) => {
                log::error!("{}", err);
            }
            Message::SelectPrev => {
                if let Some(prev_i) = self.selected_item {
                    if prev_i == 0 {
                        self.selected_item = None;
                        return text_input::focus(INPUT_ID.clone());
                    } else {
                        self.selected_item = Some(prev_i - 1);
                    }
                }
            }
            Message::SelectNext => {
                if let Some(prev_i) = self.selected_item {
                    if prev_i < 10 {
                        self.selected_item = Some(prev_i + 1);
                    }
                } else {
                    self.selected_item = Some(0);
                }
            }
            Message::Layer(e) => match e {
                LayerEvent::Focused(_) => {
                    return text_input::focus(INPUT_ID.clone());
                }
                LayerEvent::Unfocused(_) => {
                    if let Some(id) = self.active_surface {
                        return commands::layer_surface::destroy_layer_surface(id);
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
                    return commands::layer_surface::destroy_layer_surface(id);
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
                    cmds.push(text_input::focus(INPUT_ID.clone()));
                    cmds.push(commands::layer_surface::get_layer_surface(
                        SctkLayerSurfaceSettings {
                            id,
                            keyboard_interactivity: KeyboardInteractivity::Exclusive,
                            anchor: Anchor::TOP.union(Anchor::BOTTOM),
                            namespace: "launcher".into(),
                            size: (Some(600), None),
                            ..Default::default()
                        },
                    ));
                    return Command::batch(cmds);
                }
            }
            Message::Hide => {
                if let Some(id) = self.active_surface {
                    return commands::layer_surface::destroy_layer_surface(id);
                }
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

        let clear_button = button("X").padding(10).on_press(Message::Clear);

        let buttons = self
            .launcher_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let name = text(item.name.to_string())
                    .horizontal_alignment(Horizontal::Left)
                    .vertical_alignment(Vertical::Center)
                    .size(10);
                let description = text(if item.description.len() > 40 {
                    format!("{:.65}...", item.description)
                } else {
                    item.description.to_string()
                })
                .horizontal_alignment(Horizontal::Left)
                .vertical_alignment(Vertical::Center)
                .size(14);

                let mut button_content = Vec::new();
                if let Some(path) = item.category_icon.as_ref().and_then(|s| {
                    let name = match s {
                        IconSource::Name(name) | IconSource::Mime(name) => name,
                    };
                    freedesktop_icons::lookup(&name)
                        .with_theme("Pop")
                        .with_size(32)
                        .with_cache()
                        .find()
                }) {
                    if path.extension() == Some(&OsStr::new("svg")) {
                        button_content.push(
                            svg::Svg::from_path(path)
                                .width(Length::Units(16))
                                .height(Length::Units(16))
                                .style(Svg::Custom(|theme| iced_style::svg::Appearance {
                                    fill: Some(theme.palette().text),
                                }))
                                .into(),
                        )
                    } else {
                        button_content.push(
                            Image::new(path)
                                .width(Length::Units(16))
                                .height(Length::Units(16))
                                .into(),
                        )
                    }
                }

                if let Some(path) = item.icon.as_ref().and_then(|s| {
                    let name = match s {
                        IconSource::Name(name) | IconSource::Mime(name) => name,
                    };
                    freedesktop_icons::lookup(&name)
                        .with_theme("Pop")
                        .with_size(24)
                        .with_cache()
                        .find()
                }) {
                    if path.extension() == Some(&OsStr::new("svg")) {
                        button_content.push(
                            svg::Svg::from_path(path)
                                .width(Length::Units(32))
                                .height(Length::Units(32))
                                .into(),
                        )
                    } else {
                        button_content.push(
                            Image::new(path)
                                .width(Length::Units(32))
                                .height(Length::Units(32))
                                .into(),
                        )
                    }
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

        let content = column![
            row![launcher_entry, clear_button].spacing(16),
            helpers::column(buttons).spacing(16),
        ]
        .spacing(16)
        .max_width(600);

        column![
            button(vertical_space(Length::Units(1)))
                .height(Length::Fill)
                .width(Length::Fill)
                .on_press(Message::Hide)
                .style(Button::Transparent),
            widget::widget::container(content)
                .style(Container::Custom(|theme| container::Appearance {
                    text_color: Some(theme.cosmic().on_bg_color().into()),
                    background: Some(theme.extended_palette().background.base.color.into()),
                    border_radius: 16.0,
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                }))
                .padding([24, 32]),
            button(vertical_space(Length::Units(1)))
                .height(Length::Fill)
                .width(Length::Fill)
                .on_press(Message::Hide)
                .style(Button::Transparent),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(
            vec![
                dbus_toggle(0).map(|e| match e {
                    (_, LauncherDbusEvent::Toggle) => Message::Toggle,
                }),
                launcher(0).map(|(_, msg)| Message::LauncherEvent(msg)),
                events_with(|e, _status| match e {
                    cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                        wayland::Event::Layer(e),
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
                        KeyCode::Up => Some(Message::SelectPrev),
                        KeyCode::Down => Some(Message::SelectNext),
                        KeyCode::Tab if modifiers.shift() => Some(Message::SelectPrev),
                        KeyCode::Tab => Some(Message::SelectNext),
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

    fn close_requested(&self, _id: iced_sctk::application::SurfaceIdWrapper) -> Self::Message {
        Message::Closed
    }
}
