use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph, Wrap, Widget},
    Frame, Terminal,
};
use std::{
    cmp,
    error::Error,
    io,
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Sixel<'a> {
    data: &'a str,
}

impl<'a> Widget for Sixel<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // Skip entire area
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf.get_mut(x, y).set_skip(true);
            }
        }
        // ...except the first cell which "prints" all the sixel data.
        buf.get_mut(area.left(), area.top())
            .set_skip(false)
            .set_symbol(self.data);
    }
}

impl<'a> Sixel<'a> {
    pub fn data(mut self, data: &'a str) -> Sixel<'a> {
        self.data = data;
        self
    }
}

struct App {
    scroll: u16,
}

impl App {
    fn new() -> App {
        App { scroll: 0 }
    }

    fn on_tick(&mut self) {
        self.scroll += 1;
        self.scroll %= 30;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let size = f.size();

    // Scroll some text behind the image to demonstrate skipping
    let text = Paragraph::new(
        " Samara (bulerías)
Aquí metido en la trena
yo te recuerdo llorando
y me moriré de pena
mientras tú estás disfrutando
¡ay qué dolor!

Por llanto iban mis ojos
a la fuente del querer
cuanto más agua cogía
más veces quería volver

En mis sueños te llamaba
y tú no me respondías
yo en mis sueños te llamaba
y a la claridad del día
llorando me despertaba
porque yo no te veía

La que me ha dado el pañuelo
fue una gitanita mora, mora de la morería
me lo lavó en agua fría
me lo tendió en el romero
y le canté por bulerías
mientras se secó el pañuelo

Samara
fue elegida por los moros
reina de la morería
todito el pueblo la adoraba
le rezaban noche y día
porque la reina Samara
con su cara tan gitana
una Virgen parecía
ay Samara reina de la morería

Samarita sí, Samarita no
Samarita mía de mi corazón
",
    )
    .wrap(Wrap { trim: true })
    .scroll((app.scroll, 0));
    f.render_widget(text, size);

    let block = Block::default()
        .title(Span::styled(
            "Sixel",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL);

    let area = centered_rect(80, 20, size);
    f.render_widget(Clear, area); //this clears out the background
    let inner_area = block.inner(area);

    let data = std::fs::read_to_string("./assets/test.sixel").unwrap();
    let sixel = Sixel::default().data(data.as_str());

    f.render_widget(block, area);
    f.render_widget(sixel, inner_area);
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    return Rect {
        x: if width < r.width {
            r.width / 2 - width / 2
        } else {
            0
        },
        y: if height < r.height {
            r.height / 2 - height / 2
        } else {
            0
        },
        width: cmp::min(width, r.width),
        height: cmp::min(height, r.height),
    };
}
