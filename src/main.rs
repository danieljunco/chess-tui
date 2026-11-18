use std::io;
use std::net::IpAddr;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, ValueEnum};
use crossterm::event::{self, Event};

use vesper_chess::app::fixtures::demo_factory;
use vesper_chess::app::mock::{MockMode, MockProvider};
use vesper_chess::app::{App, JoinForm, ProviderFactory, Screen, StartRequest, validate_join};
use vesper_chess::domain::Color;
use vesper_chess::net::{HostProvider, NetConfig, PeerProvider};
use vesper_chess::ui::{self, theme::Theme};

/// Peer-to-peer chess for the terminal.
#[derive(Parser)]
#[command(name = "vesper", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Use ASCII letters instead of Unicode chess glyphs.
    #[arg(long, global = true)]
    ascii: bool,
    /// Disable colours (also honoured via the NO_COLOR environment variable).
    #[arg(long, global = true)]
    no_color: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Host a game in this terminal. A friend connects to you directly.
    Host {
        /// Port to listen on.
        #[arg(long, default_value_t = vesper_chess::app::DEFAULT_PORT)]
        port: u16,
        /// Address to bind. Use 0.0.0.0 to accept LAN connections.
        #[arg(long, default_value = "0.0.0.0")]
        bind: String,
        /// Colour to play.
        #[arg(long, value_enum, default_value_t = HostColor::White)]
        color: HostColor,
        /// Name shown to your opponent.
        #[arg(long, default_value = "You")]
        name: String,
    },
    /// Connect directly to a host: `vesper join 192.168.1.20:4747 K7QF-M2XA`.
    Join {
        /// Host address. The port defaults to 4747.
        address: String,
        /// Invite code from the host's waiting screen. Asked in the app if omitted.
        code: Option<String>,
        /// Name shown to the host.
        #[arg(long, default_value = "You")]
        name: String,
    },
    /// Pass-and-play on this terminal.
    Local,
    /// Visual prototype with a simulated opponent. No networking.
    Demo,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum HostColor {
    White,
    Black,
    Random,
}

impl std::fmt::Display for HostColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::White => "white",
            Self::Black => "black",
            Self::Random => "random",
        })
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let mono = cli.no_color || std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    let theme = if mono { Theme::mono() } else { Theme::dark() };
    let now = Instant::now();

    let mut app = match cli.command {
        Some(Command::Demo) => {
            let mut app = App::new(demo_factory(), now);
            app.demo = true;
            app
        }
        Some(Command::Local) => {
            let mut app = App::new(p2p_factory(NetConfig::default()), now);
            app.start(StartRequest::Local);
            app
        }
        Some(Command::Host {
            port,
            bind,
            color,
            name,
        }) => {
            let Ok(bind) = bind.parse::<IpAddr>() else {
                eprintln!("vesper: --bind needs an IP address, for example 0.0.0.0");
                return ExitCode::from(2);
            };
            let mut cfg = NetConfig::default();
            cfg.bind = bind;
            cfg.port = port;
            cfg.color = pick_color(color);
            cfg.name = name;
            let mut app = App::new(p2p_factory(cfg), now);
            app.start(StartRequest::Host);
            app
        }
        Some(Command::Join {
            address,
            code,
            name,
        }) => {
            let mut cfg = NetConfig::default();
            cfg.name = name;
            let mut app = App::new(p2p_factory(cfg), now);
            match code {
                Some(code) => match validate_join(&address, &code) {
                    Ok((addr, code)) => app.start(StartRequest::Join { addr, code }),
                    Err((_, message)) => {
                        eprintln!("vesper: {message}");
                        return ExitCode::from(2);
                    }
                },
                None => {
                    app.screen = Screen::Join(JoinForm {
                        address,
                        field: 1,
                        ..JoinForm::default()
                    });
                }
            }
            app
        }
        None => App::new(p2p_factory(NetConfig::default()), now),
    };
    app.ascii = cli.ascii;

    // `ratatui::init` installs a panic hook that restores the terminal.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, &theme);
    ratatui::restore();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("vesper: {err}");
            ExitCode::from(1)
        }
    }
}

/// Host and Join open a direct connection. Local play stays on this machine.
/// Demo mode never reaches this factory.
fn p2p_factory(cfg: NetConfig) -> ProviderFactory {
    Box::new(move |request, now| match request {
        StartRequest::Host => Box::new(HostProvider::start(cfg.clone())),
        StartRequest::Join { addr, code } => {
            Box::new(PeerProvider::start(addr, code, cfg.name.clone()))
        }
        StartRequest::Local => Box::new(MockProvider::new(MockMode::Local, now)),
    })
}

fn pick_color(color: HostColor) -> Color {
    match color {
        HostColor::White => Color::White,
        HostColor::Black => Color::Black,
        HostColor::Random => {
            let mut byte = [0u8; 1];
            getrandom::fill(&mut byte).expect("OS random");
            if byte[0].is_multiple_of(2) {
                Color::White
            } else {
                Color::Black
            }
        }
    }
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App, theme: &Theme) -> io::Result<()> {
    const FRAME: Duration = Duration::from_millis(50);
    while !app.quit {
        app.tick(Instant::now());
        terminal.draw(|frame| ui::draw(frame, app, theme))?;
        if event::poll(FRAME)? {
            match event::read()? {
                Event::Key(key) => app.on_key(key),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
    Ok(())
}
