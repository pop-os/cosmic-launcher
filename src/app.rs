use crate::{app::iced::event::listen_raw, components, fl, subscriptions::launcher};
use clap::Parser;
use cosmic::app::{Core, CosmicFlags, Settings, Task};
use cosmic::cctk::sctk;
use cosmic::dbus_activation::Details;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::event::wayland::OverlapNotifyEvent;
use cosmic::iced::event::Status;
use cosmic::iced::id::Id;
use cosmic::iced::platform_specific::runtime::wayland::{
    layer_surface::SctkLayerSurfaceSettings,
    popup::{SctkPopupSettings, SctkPositioner},
};
use cosmic::iced::platform_specific::shell::commands::{
    self,
    activation::request_token,
    layer_surface::{destroy_layer_surface, get_layer_surface, Anchor, KeyboardInteractivity},
};
use cosmic::iced::widget::{column, container, Column};
use cosmic::iced::{self, Length, Size, Subscription};
use cosmic::iced_core::keyboard::key::Named;
use cosmic::iced_core::widget::operation;
use cosmic::iced_core::{window, Border, Padding, Point, Rectangle, Shadow};
use cosmic::iced_runtime::core::event::wayland::LayerEvent;
use cosmic::iced_runtime::core::event::{wayland, PlatformSpecific};
use cosmic::iced_runtime::core::layout::Limits;
use cosmic::iced_runtime::core::window::{Event as WindowEvent, Id as SurfaceId};
use cosmic::iced_runtime::platform_specific::wayland::layer_surface::IcedMargin;
use cosmic::iced_widget::row;
use cosmic::iced_widget::scrollable::RelativeOffset;
use cosmic::iced_winit::commands::overlap_notify::overlap_notify;
use cosmic::theme::{self, Button, Container};
use cosmic::widget::icon::{from_name, IconFallback};
use cosmic::widget::id_container;
use cosmic::widget::{
    autosize, button, divider, horizontal_space, icon, mouse_area, scrollable, text,
    text_input::{self, StyleSheet as TextInputStyleSheet},
    vertical_space,
};
use cosmic::{iced_runtime, surface};
use cosmic::{keyboard_nav, Element};
use iced::keyboard::Key;
use iced::{Alignment, Color};
use pop_launcher::{ContextOption, GpuPreference, IconSource, SearchResult};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::LazyLock;
use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
    str::FromStr,
    time::Instant,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use unicode_truncate::UnicodeTruncateStr;
use unicode_width::UnicodeWidthStr;

static AUTOSIZE_ID: LazyLock<Id> = LazyLock::new(|| Id::new("autosize"));
static MAIN_ID: LazyLock<Id> = LazyLock::new(|| Id::new("main"));
static INPUT_ID: LazyLock<Id> = LazyLock::new(|| Id::new("input_id"));
static SCROLLABLE: LazyLock<Id> = LazyLock::new(|| Id::new("scrollable"));

pub(crate) static MENU_ID: LazyLock<SurfaceId> = LazyLock::new(SurfaceId::unique);
const SCROLL_MIN: usize = 8;

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Option<LauncherTasks>,
}

#[derive(Debug, Serialize, Deserialize, Clone, clap::Subcommand)]
pub enum LauncherTasks {
    #[clap(about = "Toggle the launcher and switch to the alt-tab view")]
    AltTab,
    #[clap(about = "Toggle the launcher and switch to the alt-tab view")]
    ShiftAltTab,
}

impl Display for LauncherTasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::ser::to_string(self).unwrap())
    }
}

impl FromStr for LauncherTasks {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::de::from_str(s)
    }
}

impl CosmicFlags for Args {
    type SubCommand = LauncherTasks;
    type Args = Vec<String>;

    fn action(&self) -> Option<&LauncherTasks> {
        self.subcommand.as_ref()
    }
}

pub fn run() -> cosmic::iced::Result {
    let args = Args::parse();
    cosmic::app::run_single_instance::<CosmicLauncher>(
        Settings::default()
            .antialiasing(true)
            .client_decorations(true)
            .debug(false)
            .default_text_size(16.0)
            .scale_factor(1.0)
            .no_main_window(true)
            .exit_on_close(false),
        args,
    )
}

pub fn menu_button<'a, Message>(
    content: impl Into<Element<'a, Message>>,
) -> cosmic::widget::Button<'a, Message> {
    button::custom(content)
        .class(Button::AppletMenu)
        .padding(menu_control_padding())
        .width(Length::Fill)
}

pub fn menu_control_padding() -> Padding {
    let theme = cosmic::theme::active();
    let cosmic = theme.cosmic();
    [cosmic.space_xxs(), cosmic.space_m()].into()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceState {
    Visible,
    Hidden,
    WaitingToBeShown,
}

#[derive(Clone)]
pub struct CosmicLauncher {
    core: Core,
    input_value: String,
    surface_state: SurfaceState,
    launcher_items: Vec<SearchResult>,
    tx: Option<mpsc::Sender<launcher::Request>>,
    menu: Option<(u32, Vec<ContextOption>)>,
    cursor_position: Option<Point<f32>>,
    focused: usize,
    last_hide: Instant,
    alt_tab: bool,
    window_id: window::Id,
    queue: VecDeque<Message>,
    result_ids: Vec<Id>,
    overlap: HashMap<String, Rectangle>,
    margin: f32,
    height: f32,
    needs_clear: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    Backspace,
    TabPress,
    CompleteFocusedId(Id),
    Activate(Option<usize>),
    Context(usize),
    MenuButton(u32, u32),
    CloseContextMenu,
    CursorMoved(Point<f32>),
    Hide,
    LauncherEvent(launcher::Event),
    Layer(LayerEvent),
    KeyboardNav(keyboard_nav::Action),
    ActivationToken(Option<String>, String, String, GpuPreference),
    AltTab,
    ShiftAltTab,
    Opened(Size, window::Id),
    AltRelease,
    Overlap(OverlapNotifyEvent),
    Surface(surface::Action),
}

impl CosmicLauncher {
    fn request(&self, r: launcher::Request) {
        debug!("request: {:?}", r);
        if let Some(tx) = &self.tx {
            if let Err(e) = tx.blocking_send(r) {
                error!("tx: {e}");
            }
        } else {
            info!("tx not found");
        }
    }

    fn show(&mut self) -> Task<Message> {
        self.surface_state = SurfaceState::Visible;
        self.needs_clear = true;

        Task::batch(vec![
            get_layer_surface(SctkLayerSurfaceSettings {
                id: self.window_id,
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                anchor: Anchor::TOP,
                namespace: "launcher".into(),
                size: None,
                margin: IcedMargin {
                    top: 16,
                    ..Default::default()
                },
                size_limits: Limits::NONE.min_width(1.0).min_height(1.0).max_width(600.0),
                exclusive_zone: -1,
                ..Default::default()
            }),
            overlap_notify(self.window_id, true),
        ])
    }

    fn hide(&mut self) -> Task<Message> {
        self.input_value.clear();
        self.focused = 0;
        self.alt_tab = false;
        self.queue.clear();

        self.request(launcher::Request::Close);

        let mut tasks = Vec::new();

        if self.surface_state == SurfaceState::Visible {
            tasks.push(destroy_layer_surface(self.window_id));
            if self.menu.take().is_some() {
                tasks.push(commands::popup::destroy_popup(*MENU_ID));
            }
        }

        self.surface_state = SurfaceState::Hidden;

        Task::batch(tasks)
    }

    fn focus_next(&mut self) {
        if self.launcher_items.is_empty() {
            return;
        }
        self.focused = (self.focused + 1) % self.launcher_items.len();
    }

    fn focus_previous(&mut self) {
        if self.launcher_items.is_empty() {
            return;
        }
        self.focused = (self.focused + self.launcher_items.len() - 1) % self.launcher_items.len();
    }

    fn handle_overlap(&mut self) {
        if matches!(self.surface_state, SurfaceState::Hidden) {
            return;
        }
        let mid_height = self.height / 2.;
        self.margin = 0.;

        for o in self.overlap.values() {
            if self.margin + mid_height < o.y
                || self.margin > o.y + o.height
                || mid_height < o.y + o.height / 2.0
            {
                continue;
            }
            self.margin = o.y + o.height;
        }
    }
}

async fn launch(token: Option<String>, app_id: String, exec: String, gpu: GpuPreference) {
    let mut envs = Vec::new();
    if let Some(token) = token {
        envs.push(("XDG_ACTIVATION_TOKEN".to_string(), token.clone()));
        envs.push(("DESKTOP_STARTUP_ID".to_string(), token));
    }

    if let Some(gpu_envs) = try_get_gpu_envs(gpu).await {
        envs.extend(gpu_envs);
    }

    cosmic::desktop::spawn_desktop_exec(exec, envs, Some(&app_id)).await;
}

async fn try_get_gpu_envs(gpu: GpuPreference) -> Option<HashMap<String, String>> {
    let connection = zbus::Connection::system().await.ok()?;
    let proxy = switcheroo_control::SwitcherooControlProxy::new(&connection)
        .await
        .ok()?;
    let gpus = proxy.get_gpus().await.ok()?;
    match gpu {
        GpuPreference::Default => gpus.into_iter().find(|gpu| gpu.default),
        GpuPreference::NonDefault => gpus.into_iter().find(|gpu| !gpu.default),
        GpuPreference::SpecificIdx(idx) => gpus.into_iter().nth(idx as usize),
    }
    .map(|gpu| gpu.environment)
}

impl cosmic::Application for CosmicLauncher {
    type Message = Message;
    type Executor = cosmic::executor::single::Executor;
    type Flags = Args;
    const APP_ID: &'static str = "com.system76.CosmicLauncher";

    fn init(mut core: Core, _flags: Args) -> (Self, Task<Message>) {
        core.set_keyboard_nav(false);
        (
            CosmicLauncher {
                core,
                input_value: String::new(),
                surface_state: SurfaceState::Hidden,
                launcher_items: Vec::new(),
                tx: None,
                menu: None,
                cursor_position: None,
                focused: 0,
                last_hide: Instant::now(),
                alt_tab: false,
                window_id: SurfaceId::unique(),
                queue: VecDeque::new(),
                result_ids: (0..10)
                    .map(|id| Id::new(id.to_string()))
                    .collect::<Vec<_>>(),
                margin: 0.,
                overlap: HashMap::new(),
                height: 100.,
                needs_clear: false,
            },
            Task::none(),
        )
    }

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    #[allow(clippy::too_many_lines)]
    fn update(&mut self, message: Message) -> Task<Self::Message> {
        match message {
            Message::InputChanged(value) => {
                self.input_value.clone_from(&value);
                self.request(launcher::Request::Search(value));
            }
            Message::Backspace => {
                self.input_value.pop();
                self.request(launcher::Request::Search(self.input_value.clone()));
            }
            Message::TabPress if !self.alt_tab => {
                let focused = self.focused;
                self.focused = 0;
                return cosmic::task::message(cosmic::Action::App(
                    Self::Message::CompleteFocusedId(self.result_ids[focused].clone()),
                ));
            }
            Message::TabPress => {}
            Message::CompleteFocusedId(id) => {
                let i = self
                    .result_ids
                    .iter()
                    .position(|res_id| res_id == &id)
                    .unwrap_or_default();

                if let Some(id) = self.launcher_items.get(i).map(|res| res.id) {
                    self.request(launcher::Request::Complete(id));
                }
            }
            Message::Activate(i) => {
                if let Some(item) = self.launcher_items.get(i.unwrap_or(self.focused)) {
                    self.request(launcher::Request::Activate(item.id));
                } else {
                    return self.hide();
                }
            }
            Message::Context(i) => {
                if self.menu.take().is_some() {
                    return commands::popup::destroy_popup(*MENU_ID);
                }

                if let Some(item) = self.launcher_items.get(i) {
                    self.request(launcher::Request::Context(item.id));
                }
            }
            Message::CursorMoved(pos) => {
                self.cursor_position = Some(pos);
            }
            Message::MenuButton(i, context) => {
                self.request(launcher::Request::ActivateContext(i, context));

                if self.menu.take().is_some() {
                    return commands::popup::destroy_popup(*MENU_ID);
                }
            }
            Message::Opened(size, window_id) => {
                if window_id == self.window_id {
                    self.height = size.height;
                    self.handle_overlap();
                }
            }
            Message::LauncherEvent(e) => match e {
                launcher::Event::Started(tx) => {
                    self.tx.replace(tx);
                    self.request(launcher::Request::Search(self.input_value.clone()));
                }
                launcher::Event::ServiceIsClosed => {
                    self.request(launcher::Request::ServiceIsClosed);
                }
                launcher::Event::Response(response) => match response {
                    pop_launcher::Response::Close => {
                        return self.hide();
                    }
                    #[allow(clippy::cast_possible_truncation)]
                    pop_launcher::Response::Context { id, options } => {
                        if options.is_empty() {
                            return Task::none();
                        }

                        self.menu = Some((id, options));
                        let Some(pos) = self.cursor_position.as_ref() else {
                            return Task::none();
                        };
                        let rect = Rectangle {
                            x: pos.x.round() as i32,
                            y: pos.y.round() as i32,
                            width: 1,
                            height: 1,
                        };
                        return commands::popup::get_popup(SctkPopupSettings {
                                    parent: self.window_id,
                                    id: *MENU_ID,
                                    positioner: SctkPositioner {
                                        size: None,
                                        size_limits: Limits::NONE.min_width(1.0).min_height(1.0).max_width(300.0).max_height(800.0),
                                        anchor_rect: rect,
                                        anchor:
                                            sctk::reexports::protocols::xdg::shell::client::xdg_positioner::Anchor::Right,
                                        gravity: sctk::reexports::protocols::xdg::shell::client::xdg_positioner::Gravity::Right,
                                        reactive: true,
                                        ..Default::default()
                                    },
                                    grab: true,
                                    parent_size: None,
                                    close_with_children: false,
                                    input_zone: None,
                                });
                    }
                    pop_launcher::Response::DesktopEntry {
                        path,
                        gpu_preference,
                        action_name,
                    } => {
                        if let Some(entry) = cosmic::desktop::load_desktop_file(None, path) {
                            let exec = if let Some(action_name) = action_name {
                                entry
                                    .desktop_actions
                                    .into_iter()
                                    .find(|action| action.name == action_name)
                                    .map(|action| action.exec)
                            } else {
                                entry.exec
                            };

                            let Some(exec) = exec else {
                                return Task::none();
                            };
                            return request_token(
                                Some(String::from(Self::APP_ID)),
                                Some(self.window_id),
                            )
                            .map(move |token| {
                                cosmic::Action::App(Message::ActivationToken(
                                    token,
                                    entry.id.to_string(),
                                    exec.clone(),
                                    gpu_preference,
                                ))
                            });
                        }
                    }
                    pop_launcher::Response::Update(mut list) => {
                        if self.alt_tab && list.is_empty() {
                            return self.hide();
                        }
                        list.sort_by(|a, b| {
                            let a = i32::from(a.window.is_none());
                            let b = i32::from(b.window.is_none());
                            a.cmp(&b)
                        });
                        self.launcher_items.splice(.., list);
                        if self.result_ids.len() < self.launcher_items.len() {
                            self.result_ids.extend(
                                (self.result_ids.len()..self.launcher_items.len())
                                    .map(|id| Id::new((id).to_string()))
                                    .collect::<Vec<_>>(),
                            );
                        }
                        let mut cmds = Vec::new();

                        while let Some(element) = self.queue.pop_front() {
                            let updated = self.update(element);
                            cmds.push(updated);
                        }

                        if self.surface_state == SurfaceState::WaitingToBeShown {
                            cmds.push(self.show());
                        }
                        return Task::batch(cmds);
                    }
                    pop_launcher::Response::Fill(s) => {
                        self.input_value = s;
                        self.request(launcher::Request::Search(self.input_value.clone()));
                    }
                },
            },
            Message::Layer(e) => match e {
                LayerEvent::Focused | LayerEvent::Done => {}
                LayerEvent::Unfocused => {
                    self.last_hide = Instant::now();
                    return self.hide();
                }
            },
            Message::Overlap(overlap_notify_event) => match overlap_notify_event {
                OverlapNotifyEvent::OverlapLayerAdd {
                    identifier,
                    namespace,
                    logical_rect,
                    exclusive,
                    ..
                } => {
                    if self.needs_clear {
                        self.needs_clear = false;
                        self.overlap.clear();
                    }
                    if exclusive > 0 || namespace == "Dock" || namespace == "Panel" {
                        self.overlap.insert(identifier, logical_rect);
                    }
                    self.handle_overlap();
                }
                OverlapNotifyEvent::OverlapLayerRemove { identifier } => {
                    self.overlap.remove(&identifier);
                    self.handle_overlap();
                }
                _ => {}
            },
            Message::CloseContextMenu => {
                if self.menu.take().is_some() {
                    return commands::popup::destroy_popup(*MENU_ID);
                }
            }
            Message::Hide => {
                if self.menu.take().is_some() {
                    return commands::popup::destroy_popup(*MENU_ID);
                }
                return self.hide();
            }
            Message::KeyboardNav(e) => {
                match e {
                    keyboard_nav::Action::FocusNext => {
                        self.focus_next();
                        // TODO ideally we could use an operation to scroll exactly to a specific widget.
                        return iced_runtime::task::widget(operation::scrollable::snap_to(
                            SCROLLABLE.clone(),
                            RelativeOffset {
                                x: 0.,
                                y: (self.focused as f32
                                    / (self.launcher_items.len() as f32 - 1.).max(1.))
                                .max(0.0),
                            },
                        ));
                    }
                    keyboard_nav::Action::FocusPrevious => {
                        self.focus_previous();
                        return iced_runtime::task::widget(operation::scrollable::snap_to(
                            SCROLLABLE.clone(),
                            RelativeOffset {
                                x: 0.,
                                y: (self.focused as f32
                                    / (self.launcher_items.len() as f32 - 1.).max(1.))
                                .max(0.0),
                            },
                        ));
                    }
                    keyboard_nav::Action::Escape => {
                        self.input_value.clear();
                        self.request(launcher::Request::Search(String::new()));
                    }
                    _ => {}
                };
            }
            Message::ActivationToken(token, app_id, exec, dgpu) => {
                return Task::perform(launch(token, app_id, exec, dgpu), |()| {
                    cosmic::action::app(Message::Hide)
                });
            }
            Message::AltTab => {
                self.focus_next();
                return iced_runtime::task::widget(operation::scrollable::snap_to(
                    SCROLLABLE.clone(),
                    RelativeOffset {
                        x: 0.,
                        y: (self.focused as f32 / (self.launcher_items.len() as f32 - 1.).max(1.))
                            .max(0.0),
                    },
                ));
            }
            Message::ShiftAltTab => {
                self.focus_previous();
                return iced_runtime::task::widget(operation::scrollable::snap_to(
                    SCROLLABLE.clone(),
                    RelativeOffset {
                        x: 0.,
                        y: (self.focused as f32 / (self.launcher_items.len() as f32 - 1.).max(1.))
                            .max(0.0),
                    },
                ));
            }
            Message::AltRelease => {
                if self.alt_tab {
                    return self.update(Message::Activate(None));
                }
            }
            Message::Surface(a) => {
                return cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Surface(
                    a,
                )))
            }
        }
        Task::none()
    }

    fn dbus_activation(
        &mut self,
        msg: cosmic::dbus_activation::Message,
    ) -> iced::Task<cosmic::Action<Self::Message>> {
        match msg.msg {
            Details::Activate => {
                if self.surface_state != SurfaceState::Hidden {
                    return self.hide();
                }
                // hack: allow to close the launcher from the panel button
                if self.last_hide.elapsed().as_millis() > 100 {
                    self.request(launcher::Request::Search(String::new()));

                    self.surface_state = SurfaceState::WaitingToBeShown;
                    return Task::none();
                }
            }
            Details::ActivateAction { action, .. } => {
                debug!("ActivateAction {}", action);

                let Ok(cmd) = LauncherTasks::from_str(&action) else {
                    return Task::none();
                };

                if self.surface_state == SurfaceState::Hidden {
                    self.surface_state = SurfaceState::WaitingToBeShown;
                }

                match cmd {
                    LauncherTasks::AltTab => {
                        if self.alt_tab {
                            return self.update(Message::AltTab);
                        }

                        self.alt_tab = true;
                        self.request(launcher::Request::Search(String::new()));
                        self.queue.push_back(Message::AltTab);
                    }
                    LauncherTasks::ShiftAltTab => {
                        if self.alt_tab {
                            return self.update(Message::ShiftAltTab);
                        }

                        self.alt_tab = true;
                        self.request(launcher::Request::Search(String::new()));
                        self.queue.push_back(Message::ShiftAltTab);
                    }
                }
            }
            Details::Open { .. } => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<Self::Message> {
        unreachable!("No main window")
    }

    #[allow(clippy::too_many_lines)]
    fn view_window(&self, id: SurfaceId) -> Element<Self::Message> {
        if id == self.window_id {
            let launcher_entry = text_input::search_input(fl!("type-to-search"), &self.input_value)
                .on_input(Message::InputChanged)
                .on_paste(Message::InputChanged)
                .on_submit(|_| Message::Activate(None))
                .style(cosmic::theme::TextInput::Custom {
                    active: Box::new(|theme| theme.focused(&cosmic::theme::TextInput::Search)),
                    error: Box::new(|theme| theme.focused(&cosmic::theme::TextInput::Search)),
                    hovered: Box::new(|theme| theme.focused(&cosmic::theme::TextInput::Search)),
                    focused: Box::new(|theme| theme.focused(&cosmic::theme::TextInput::Search)),
                    disabled: Box::new(|theme| theme.disabled(&cosmic::theme::TextInput::Search)),
                })
                .width(600)
                .id(INPUT_ID.clone())
                .always_active();

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

                    let name = Column::with_children(name.lines().map(|line| {
                        text::body(if line.width() > 60 {
                            format!("{}...", line.unicode_truncate(60).0)
                        } else {
                            line.to_string()
                        })
                        .align_x(Horizontal::Left)
                        .align_y(Vertical::Center)
                        .class(cosmic::theme::Text::Custom(|t| {
                            cosmic::iced::widget::text::Style {
                                color: Some(t.cosmic().on_bg_color().into()),
                            }
                        }))
                        .into()
                    }));

                    let desc = Column::with_children(desc.lines().map(|line| {
                        text::caption(if line.width() > 80 {
                            format!("{}...", line.unicode_truncate(80).0)
                        } else {
                            line.to_string()
                        })
                        .align_x(Horizontal::Left)
                        .align_y(Vertical::Center)
                        .class(theme::Text::Custom(|t| cosmic::iced::widget::text::Style {
                            color: Some(t.cosmic().on_bg_color().into()),
                        }))
                        .into()
                    }));

                    let mut button_content = Vec::new();
                    if !self.alt_tab {
                        if let Some(source) = item.category_icon.as_ref() {
                            let name = match source {
                                IconSource::Name(name) | IconSource::Mime(name) => name,
                            };
                            button_content.push(
                                icon(from_name(name.clone()).into())
                                    .width(Length::Fixed(16.0))
                                    .height(Length::Fixed(16.0))
                                    .class(cosmic::theme::Svg::Custom(Rc::new(|theme| {
                                        cosmic::iced::widget::svg::Style {
                                            color: Some(theme.cosmic().on_bg_color().into()),
                                        }
                                    })))
                                    .into(),
                            );
                        }
                    }
                    if let Some(source) = item.icon.as_ref() {
                        let name = match source {
                            IconSource::Name(name) | IconSource::Mime(name) => name,
                        };
                        button_content.push(
                            icon(
                                from_name(name.clone())
                                    .size(64)
                                    .fallback(Some(IconFallback::Names(vec![
                                        "application-default".into(),
                                        "application-x-executable".into(),
                                    ])))
                                    .into(),
                            )
                            .width(Length::Fixed(32.0))
                            .height(Length::Fixed(32.0))
                            .into(),
                        );
                    }

                    button_content.push(column![name, desc].width(Length::FillPortion(5)).into());
                    if i < 10 {
                        button_content.push(
                            container(
                                text::body(format!("Ctrl + {}", (i + 1) % 10))
                                    .align_y(Vertical::Center)
                                    .align_x(Horizontal::Right)
                                    .class(theme::Text::Custom(|t| {
                                        cosmic::iced::widget::text::Style {
                                            color: Some(t.cosmic().on_bg_color().into()),
                                        }
                                    })),
                            )
                            .width(Length::FillPortion(1))
                            .center_y(Length::Shrink)
                            .align_y(Vertical::Center)
                            .align_x(Horizontal::Right)
                            .into(),
                        );
                    }
                    let is_focused = i == self.focused;
                    let btn = mouse_area(
                        cosmic::widget::button::custom(
                            row(button_content).spacing(8).align_y(Alignment::Center),
                        )
                        .id(self.result_ids[i].clone())
                        .width(Length::Fill)
                        .on_press(Message::Activate(Some(i)))
                        .padding([8, 24])
                        .class(Button::Custom {
                            active: Box::new(move |focused, theme| {
                                let focused = is_focused || focused;
                                let rad_s = theme.cosmic().corner_radii.radius_s;
                                let a = if focused {
                                    button::Catalog::hovered(theme, focused, focused, &Button::Text)
                                } else {
                                    button::Catalog::active(theme, focused, focused, &Button::Text)
                                };
                                button::Style {
                                    border_radius: rad_s.into(),
                                    outline_width: 0.0,
                                    ..a
                                }
                            }),
                            hovered: Box::new(move |focused, theme| {
                                let focused = is_focused || focused;
                                let rad_s = theme.cosmic().corner_radii.radius_s;

                                let text = button::Catalog::hovered(
                                    theme,
                                    focused,
                                    focused,
                                    &Button::Text,
                                );
                                button::Style {
                                    border_radius: rad_s.into(),
                                    outline_width: 0.0,
                                    ..text
                                }
                            }),
                            disabled: Box::new(|theme| {
                                let rad_s = theme.cosmic().corner_radii.radius_s;

                                let text = button::Catalog::disabled(theme, &Button::Text);
                                button::Style {
                                    border_radius: rad_s.into(),
                                    outline_width: 0.0,
                                    ..text
                                }
                            }),
                            pressed: Box::new(move |focused, theme| {
                                let focused = is_focused || focused;
                                let rad_s = theme.cosmic().corner_radii.radius_s;

                                let text = button::Catalog::pressed(
                                    theme,
                                    focused,
                                    focused,
                                    &Button::Text,
                                );
                                button::Style {
                                    border_radius: rad_s.into(),
                                    outline_width: 0.0,
                                    ..text
                                }
                            }),
                        }),
                    )
                    .on_right_release(Message::Context(i));
                    if i == self.launcher_items.len() - 1 {
                        vec![btn.into()]
                    } else {
                        vec![btn.into(), divider::horizontal::light().into()]
                    }
                })
                .collect();

            let mut content = if self.alt_tab {
                Column::new()
                    .max_width(600)
                    .spacing(16)
                    .width(Length::Shrink)
                    .height(Length::Shrink)
            } else {
                column![launcher_entry]
                    .max_width(600)
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .spacing(16)
            };

            if buttons.len() > SCROLL_MIN {
                content = content.push(
                    container(scrollable(components::list::column(buttons)).id(SCROLLABLE.clone()))
                        .max_height(504),
                );
            } else if !buttons.is_empty() {
                content = content.push(components::list::column(buttons));
            };

            let window = Column::new()
                .push(vertical_space().height(Length::Fixed(self.margin)))
                .push(
                    container(id_container(content, MAIN_ID.clone()))
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .class(Container::Custom(Box::new(|theme| container::Style {
                            text_color: Some(theme.cosmic().on_bg_color().into()),
                            icon_color: Some(theme.cosmic().on_bg_color().into()),
                            background: Some(Color::from(theme.cosmic().background.base).into()),
                            border: Border {
                                radius: theme.cosmic().corner_radii.radius_m.into(),
                                width: 1.0,
                                color: theme.cosmic().bg_divider().into(),
                            },
                            shadow: Shadow::default(),
                        })))
                        .padding([24, 32]),
                );

            let autosize = autosize::autosize(
                if self.menu.is_some() {
                    Element::from(
                        mouse_area(window)
                            .on_release(Message::CloseContextMenu)
                            .on_right_release(Message::CloseContextMenu),
                    )
                } else {
                    window.into()
                },
                AUTOSIZE_ID.clone(),
            );
            return Element::from(autosize);
        }
        if id == *MENU_ID {
            let Some((i, options)) = self.menu.as_ref() else {
                return container(horizontal_space().width(Length::Fixed(1.0)))
                    .width(Length::Fixed(1.0))
                    .height(Length::Fixed(1.0))
                    .into();
            };
            let list_column = Column::with_children(options.iter().map(|option| {
                menu_button(text::body(&option.name))
                    .on_press(Message::MenuButton(*i, option.id))
                    .into()
            }))
            .padding([8, 0]);

            return container(
                container(scrollable(list_column)).class(theme::Container::custom(|theme| {
                    let cosmic = theme.cosmic();
                    let corners = cosmic.corner_radii;
                    container::Style {
                        text_color: Some(cosmic.background.on.into()),
                        background: Some(Color::from(cosmic.background.base).into()),
                        border: Border {
                            radius: corners.radius_m.into(),
                            width: 1.0,
                            color: cosmic.background.divider.into(),
                        },
                        shadow: Shadow::default(),
                        icon_color: Some(cosmic.background.on.into()),
                    }
                })),
            )
            .width(Length::Shrink)
            .height(Length::Shrink)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Top)
            .into();
        }

        vertical_space().height(Length::Fixed(1.0)).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            launcher::subscription(0).map(Message::LauncherEvent),
            listen_raw(|e, status, id| match e {
                cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                    wayland::Event::Layer(e, ..),
                )) => Some(Message::Layer(e)),
                cosmic::iced::Event::PlatformSpecific(PlatformSpecific::Wayland(
                    wayland::Event::OverlapNotify(event),
                )) => Some(Message::Overlap(event)),
                cosmic::iced::Event::Keyboard(iced::keyboard::Event::KeyReleased {
                    key: Key::Named(Named::Alt | Named::Super),
                    ..
                }) => Some(Message::AltRelease),
                cosmic::iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key,
                    text: _,
                    modifiers,
                    ..
                }) => match key {
                    Key::Character(c) if modifiers.control() && (c == "p" || c == "k") => {
                        Some(Message::KeyboardNav(keyboard_nav::Action::FocusPrevious))
                    }
                    Key::Character(c) if modifiers.control() && (c == "n" || c == "j") => {
                        Some(Message::KeyboardNav(keyboard_nav::Action::FocusNext))
                    }
                    Key::Character(c) if modifiers.control() => {
                        let nums = (0..10)
                            .map(|n| (n.to_string(), ((n + 10) % 10) - 1))
                            .collect::<Vec<_>>();
                        nums.iter()
                            .find_map(|n| (n.0 == c).then(|| Message::Activate(Some(n.1))))
                    }
                    Key::Named(Named::ArrowUp) => {
                        Some(Message::KeyboardNav(keyboard_nav::Action::FocusPrevious))
                    }
                    Key::Named(Named::ArrowDown) => {
                        Some(Message::KeyboardNav(keyboard_nav::Action::FocusNext))
                    }
                    Key::Named(Named::Escape) => Some(Message::Hide),
                    Key::Named(Named::Tab) => Some(Message::TabPress),
                    Key::Named(Named::Backspace)
                        if matches!(status, Status::Ignored) && modifiers.is_empty() =>
                    {
                        Some(Message::Backspace)
                    }
                    _ => None,
                },
                cosmic::iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    Some(Message::CursorMoved(position))
                }
                cosmic::iced::Event::Window(WindowEvent::Opened { position: _, size }) => {
                    Some(Message::Opened(size, id))
                }
                cosmic::iced::Event::Window(WindowEvent::Resized(s)) => {
                    Some(Message::Opened(s, id))
                }
                _ => None,
            }),
        ])
    }
}
