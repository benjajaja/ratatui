use image::{imageops::FilterType, DynamicImage};
use ratatui::backend::TermionBackend;
use ratatui::{
    backend::Backend,
    buffer::Buffer,
    layout::Rect,
    widgets::{Paragraph, Widget, Wrap},
    Frame, Terminal,
};
use sixel_rs::{
    encoder::{Encoder, QuickFrameBuilder},
    optflags::EncodePolicy,
    sys::PixelFormat,
};
use std::{error::Error, fs, io, path::Path, sync::mpsc, thread, time::Duration};
use termion::{
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::IntoAlternateScreen,
    terminal_size, terminal_size_pixels,
};

struct Image {
    data: String,
    size: (u16, u16),
}

impl Widget for &Image {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // skip entire area
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf.get_mut(x, y).set_skip(true);
            }
        }
        // ...except the first cell which "prints" all the sixel data.
        buf.get_mut(area.left(), area.top())
            .set_skip(false)
            .set_symbol(self.data.as_str());
    }
}

const TMP_FILE: &'static str = "./assets/test_out.sixel";
impl From<DynamicImage> for Image {
    fn from(img: DynamicImage) -> Image {
        // Image must be resized precisely in steps of cell width/height.
        // Otherwise the cells underlying the right / bottom edges could be showing the characters
        // from previous draw calls where the image does not cover the cells.
        let (img, size_cells) = resize_to_terminal(img);

        // encode as sixel data
        let (w, h) = (img.width(), img.height());
        let bytes = img.to_rgb8().as_raw().to_vec();
        let encoder = Encoder::new().unwrap();
        encoder.set_encode_policy(EncodePolicy::Fast).unwrap();
        let frame = QuickFrameBuilder::new()
            .width(w as _)
            .height(h as _)
            .format(PixelFormat::RGB888)
            .pixels(bytes);

        // sixel-rs only supports output-to-file
        encoder.set_output(Path::new(TMP_FILE)).unwrap();
        encoder.encode_bytes(frame).unwrap();

        // ...so we just read it back
        let data = fs::read_to_string(TMP_FILE).unwrap();
        fs::remove_file(TMP_FILE).unwrap();
        Image {
            data,
            size: size_cells,
        }
    }
}

fn resize_to_terminal(img: DynamicImage) -> (DynamicImage, (u16, u16)) {
    let (cols, rows) = terminal_size().unwrap();
    let (width, height) = terminal_size_pixels().unwrap();
    let cell_width = width / cols;
    let cell_height = height / rows;

    let (width, height) = (img.width() as u16, img.height() as u16);
    let resize_w = width - (width % cell_width);
    let resize_h = height - (height % cell_height);
    let img = img.resize_to_fill(resize_w as _, resize_h as _, FilterType::Nearest);
    let size_cells = (resize_w / cell_width, resize_h / cell_height);
    (img, size_cells)
}

struct App {
    scroll: u16,
    image: Image,
}

impl App {
    fn new() -> App {
        // encode and resize only once
        let image: Image = image::io::Reader::open("./assets/Ada.png")
            .unwrap()
            .decode()
            .unwrap()
            .into();

        App { scroll: 0, image }
    }

    fn on_tick(&mut self) {
        self.scroll += 1;
        self.scroll %= 30;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let app = App::new();
    let tick_rate = Duration::from_millis(500);
    return run(app, tick_rate);
}

fn run(mut app: App, tick_rate: Duration) -> Result<(), Box<dyn Error>> {
    // Crossterm does not have a function to query pixel size, but it would be possible to use the
    // termwiz or termion crate just for the query. Crossterm does render sixels without issues.
    // Termwiz does have one, but sixels don't render correctly.
    // Only termion renders correctly and supports querying pixel size.
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

    // scroll some text behind the image to demonstrate proper skipping
    let text = Paragraph::new(include_str!("../assets/samara.txt"))
        .wrap(Wrap { trim: true })
        .scroll((app.scroll, 0));
    f.render_widget(text, size);

    let area = Rect::new(1, 2, app.image.size.0, app.image.size.1).intersection(size);
    f.render_widget(&app.image, area);
}
