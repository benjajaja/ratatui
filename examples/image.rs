use image::{imageops::FilterType, DynamicImage};
use ratatui::backend::TermionBackend;
use ratatui::{
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
    Frame, Terminal,
};
use sixel_rs::{
    encoder::{Encoder, QuickFrameBuilder},
    optflags::EncodePolicy,
    sys::PixelFormat,
};
use std::fs;
use std::{cmp, error::Error, io, path::Path, sync::mpsc, thread, time::Duration};
use termion::{
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::IntoAlternateScreen,
    terminal_size, terminal_size_pixels,
};

struct Image {
    data: String,
    rect: Rect,
}

impl Widget for &Image {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // Skip entire area
        for y in self.rect.top()..self.rect.bottom() {
            for x in self.rect.left()..self.rect.right() {
                buf.get_mut(x, y).set_skip(true);
            }
        }
        // ...except the first cell which "prints" all the sixel data.
        buf.get_mut(self.rect.left(), self.rect.top())
            .set_skip(false)
            .set_symbol(self.data.as_str());
    }
}

impl From<Sixel> for Image {
    fn from(sixel: Sixel) -> Image {
        Image {
            data: sixel.data,
            rect: sixel.rect,
        }
    }
}

struct Sixel {
    data: String,
    rect: Rect,
}

const TMP_FILE: &'static str = "./assets/test_out.sixel";
impl From<DynamicImage> for Sixel {
    fn from(img: DynamicImage) -> Sixel {
        let (img, rect) = resize_to_terminal(img);
        let (w, h) = (img.width(), img.height());
        let bytes = img.to_rgb8().as_raw().to_vec();
        let encoder = Encoder::new().unwrap();
        encoder.set_output(Path::new(TMP_FILE)).unwrap();
        encoder.set_encode_policy(EncodePolicy::Fast).unwrap();
        let frame = QuickFrameBuilder::new()
            .width(w as _)
            .height(h as _)
            .format(PixelFormat::RGB888)
            .pixels(bytes);

        encoder.encode_bytes(frame).unwrap();

        let data = fs::read_to_string(TMP_FILE).unwrap();
        fs::remove_file(TMP_FILE).unwrap();
        Sixel { data, rect }
    }
}

fn resize_to_terminal(img: DynamicImage) -> (DynamicImage, Rect) {
    let (cols, rows) = terminal_size().unwrap();
    let (width, height) = terminal_size_pixels().unwrap();
    let char_width = (width / cols) as u32;
    let char_height = (height / rows) as u32;
    let resize_w = img.width() - (img.width() % char_width);
    let resize_h = img.height() - (img.height() % char_height);
    let rect = Rect::new(
        0,
        0,
        (resize_w / char_width).try_into().unwrap(),
        (resize_h / char_height).try_into().unwrap(),
    );
    (
        img.resize_to_fill(resize_w, resize_h, FilterType::Nearest),
        rect,
    )
}

struct App {
    scroll: u16,
    image: Image,
}

impl App {
    fn new() -> App {
        let img = image::io::Reader::open("./assets/Ada.png")
            .unwrap()
            .decode()
            .unwrap();

        let sixel: Sixel = img.into();
        App {
            scroll: 0,
            image: sixel.into(),
        }
    }

    fn on_tick(&mut self) {
        self.scroll += 1;
        self.scroll %= 10;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let app = App::new();
    let tick_rate = Duration::from_millis(250);
    return run(app, tick_rate);
}

fn run(mut app: App, tick_rate: Duration) -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout()
        .into_raw_mode()
        .unwrap()
        .into_alternate_screen()
        .unwrap();
    let stdout = MouseTerminal::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = events(tick_rate);
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        match events.recv()? {
            Event::Input(key) => match key {
                Key::Char('q') => return Ok(()),
                _ => {}
            },
            Event::Tick => app.on_tick(),
        }
    }
}

enum Event {
    Input(Key),
    Tick,
}

fn events(tick_rate: Duration) -> mpsc::Receiver<Event> {
    let (tx, rx) = mpsc::channel();
    let keys_tx = tx.clone();
    thread::spawn(move || {
        let stdin = io::stdin();
        for key in stdin.keys().flatten() {
            if let Err(err) = keys_tx.send(Event::Input(key)) {
                eprintln!("{err}");
                return;
            }
        }
    });
    thread::spawn(move || loop {
        if let Err(err) = tx.send(Event::Tick) {
            eprintln!("{err}");
            break;
        }
        thread::sleep(tick_rate);
    });
    rx
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

    // let sixel = Sixel::default().data(app.sixel_data);
    f.render_widget(block, area);
    f.render_widget(&app.image, inner_area);
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    return Rect {
        x: center(width, r.width),
        y: center(height, r.height),
        width: cmp::min(width, r.width),
        height: cmp::min(height, r.height),
    };
}

fn center(a: u16, b: u16) -> u16 {
    if a < b {
        b / 2 - a / 2
    } else {
        0
    }
}
