//! # [Ratatui] Constraint explorer example
//!
//! The latest version of this example is available in the [examples] folder in the repository.
//!
//! Please note that the examples are designed to be run against the `main` branch of the Github
//! repository. This means that you may not be able to compile with the latest release version on
//! crates.io, or the one that you have installed locally.
//!
//! See the [examples readme] for more information on finding examples that match the version of the
//! library you are using.
//!
//! [Ratatui]: https://github.com/ratatui-org/ratatui
//! [examples]: https://github.com/ratatui-org/ratatui/blob/main/examples
//! [examples readme]: https://github.com/ratatui-org/ratatui/blob/main/examples/README.md

use std::io::{self, stdout};

use color_eyre::{config::HookBuilder, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use itertools::Itertools;
use ratatui::{
    layout::{Constraint::*, Flex},
    prelude::*,
    style::palette::tailwind::*,
    symbols::line,
    widgets::*,
};
use strum::{Display, EnumIter, FromRepr};

#[derive(Default)]
struct App {
    mode: AppMode,
    spacing: u16,
    constraints: Vec<Constraint>,
    selected_index: usize,
    value: u16,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum AppMode {
    #[default]
    Running,
    Quit,
}

/// A variant of [`Constraint`] that can be rendered as a tab.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, EnumIter, FromRepr, Display)]
enum ConstraintName {
    #[default]
    Length,
    Percentage,
    Ratio,
    Min,
    Max,
    Fill,
}

/// A widget that renders a [`Constraint`] as a block. E.g.:
/// ```plain
/// ┌──────────────┐
/// │  Length(16)  │
/// │     16px     │
/// └──────────────┘
/// ```
struct ConstraintBlock {
    selected: bool,
    legend: bool,
    constraint: Constraint,
}

/// A widget that renders a spacer with a label indicating the width of the spacer. E.g.:
///
/// ```plain
/// ┌      ┐
///   8 px
/// └      ┘
/// ```
struct SpacerBlock;

fn main() -> Result<()> {
    init_error_hooks()?;
    let terminal = init_terminal()?;
    App::default().run(terminal)?;
    restore_terminal()?;
    Ok(())
}

// App behaviour
impl App {
    fn run(&mut self, mut terminal: Terminal<impl Backend>) -> Result<()> {
        self.insert_test_defaults();

        while self.is_running() {
            self.draw(&mut terminal)?;
            self.handle_events()?;
        }
        Ok(())
    }

    // TODO remove these - these are just for testing
    fn insert_test_defaults(&mut self) {
        self.constraints = vec![
            Constraint::Length(20),
            Constraint::Length(20),
            Constraint::Length(20),
        ];
        self.value = 20;
    }

    fn is_running(&self) -> bool {
        self.mode == AppMode::Running
    }

    fn draw(&self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        terminal.draw(|frame| frame.render_widget(self, frame.size()))?;
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        use KeyCode::*;
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                Char('q') | Esc => self.exit(),
                Char('1') => self.swap_constraint(ConstraintName::Min),
                Char('2') => self.swap_constraint(ConstraintName::Max),
                Char('3') => self.swap_constraint(ConstraintName::Length),
                Char('4') => self.swap_constraint(ConstraintName::Percentage),
                Char('5') => self.swap_constraint(ConstraintName::Ratio),
                Char('6') => self.swap_constraint(ConstraintName::Fill),
                Char('+') => self.increment_spacing(),
                Char('-') => self.decrement_spacing(),
                Char('x') => self.delete_block(),
                Char('a') => self.insert_block(),
                Char('k') | Up => self.increment_value(),
                Char('j') | Down => self.decrement_value(),
                Char('h') | Left => self.prev_block(),
                Char('l') | Right => self.next_block(),
                _ => {}
            },
            _ => {}
        }
        Ok(())
    }

    /// select the next block with wrap around
    fn increment_value(&mut self) {
        if self.constraints.is_empty() {
            return;
        }
        self.constraints[self.selected_index] = match self.constraints[self.selected_index] {
            Constraint::Length(v) => Constraint::Length(v.saturating_add(1)),
            Constraint::Min(v) => Constraint::Min(v.saturating_add(1)),
            Constraint::Max(v) => Constraint::Max(v.saturating_add(1)),
            Constraint::Fill(v) => Constraint::Fill(v.saturating_add(1)),
            Constraint::Percentage(v) => Constraint::Percentage(v.saturating_add(1)),
            Constraint::Ratio(n, d) => Constraint::Ratio(n, d.saturating_add(1)),
        };
    }

    fn decrement_value(&mut self) {
        if self.constraints.is_empty() {
            return;
        }
        self.constraints[self.selected_index] = match self.constraints[self.selected_index] {
            Constraint::Length(v) => Constraint::Length(v.saturating_sub(1)),
            Constraint::Min(v) => Constraint::Min(v.saturating_sub(1)),
            Constraint::Max(v) => Constraint::Max(v.saturating_sub(1)),
            Constraint::Fill(v) => Constraint::Fill(v.saturating_sub(1)),
            Constraint::Percentage(v) => Constraint::Percentage(v.saturating_sub(1)),
            Constraint::Ratio(n, d) => Constraint::Ratio(n, d.saturating_sub(1)),
        };
    }

    /// select the next block with wrap around
    fn next_block(&mut self) {
        if self.constraints.is_empty() {
            return;
        }
        let len = self.constraints.len();
        self.selected_index = (self.selected_index + 1) % len;
    }

    /// select the previous block with wrap around
    fn prev_block(&mut self) {
        if self.constraints.is_empty() {
            return;
        }
        let len = self.constraints.len();
        self.selected_index = (self.selected_index + self.constraints.len() - 1) % len;
    }

    /// delete the selected block
    fn delete_block(&mut self) {
        if self.constraints.is_empty() {
            return;
        }
        self.constraints.remove(self.selected_index);
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    /// insert a block after the selected block
    fn insert_block(&mut self) {
        let index = self
            .selected_index
            .saturating_add(1)
            .min(self.constraints.len());
        let constraint = Constraint::Length(self.value);
        self.constraints.insert(index, constraint);
        self.selected_index = index;
    }

    fn increment_spacing(&mut self) {
        self.spacing = self.spacing.saturating_add(1);
    }

    fn decrement_spacing(&mut self) {
        self.spacing = self.spacing.saturating_sub(1);
    }

    fn exit(&mut self) {
        self.mode = AppMode::Quit
    }

    fn swap_constraint(&mut self, name: ConstraintName) {
        if self.constraints.is_empty() {
            return;
        }
        let constraint = match name {
            ConstraintName::Length => Length(self.value),
            ConstraintName::Percentage => Percentage(self.value),
            ConstraintName::Min => Min(self.value),
            ConstraintName::Max => Max(self.value),
            ConstraintName::Fill => Fill(self.value),
            ConstraintName::Ratio => Ratio(1, self.value as u32 / 4), // for balance
        };
        self.constraints[self.selected_index] = constraint;
    }
}

impl From<Constraint> for ConstraintName {
    fn from(constraint: Constraint) -> Self {
        use Constraint::*;
        match constraint {
            Length(_) => ConstraintName::Length,
            Percentage(_) => ConstraintName::Percentage,
            Ratio(_, _) => ConstraintName::Ratio,
            Min(_) => ConstraintName::Min,
            Max(_) => ConstraintName::Max,
            Fill(_) => ConstraintName::Fill,
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, instructions_area, swap_legend_area, _, blocks_area] =
            area.split(&Layout::vertical([
                Length(2), // header
                Length(2), // instructions
                Length(1), // swap key legend
                Length(1), // gap
                Fill(1),   // blocks
            ]));

        self.header().render(header_area, buf);
        self.instructions().render(instructions_area, buf);
        self.swap_legend().render(swap_legend_area, buf);
        self.render_layout_blocks(blocks_area, buf);
    }
}

// App rendering
impl App {
    const HEADER_COLOR: Color = SLATE.c200;
    const TEXT_COLOR: Color = SLATE.c400;
    const AXIS_COLOR: Color = SLATE.c500;

    fn header(&self) -> impl Widget {
        let text = "Constraint Explorer";
        text.bold().fg(Self::HEADER_COLOR).to_centered_line()
    }

    fn instructions(&self) -> impl Widget {
        let text = "◄ ►: select, ▲ ▼: edit, 1-6: swap, a: add, x: delete, q: quit, + -: spacing";
        Paragraph::new(text)
            .fg(Self::TEXT_COLOR)
            .centered()
            .wrap(Wrap { trim: false })
    }

    fn swap_legend(&self) -> impl Widget {
        #[allow(unstable_name_collisions)]
        Paragraph::new(
            Line::from(
                [
                    ConstraintName::Min,
                    ConstraintName::Max,
                    ConstraintName::Length,
                    ConstraintName::Percentage,
                    ConstraintName::Ratio,
                    ConstraintName::Fill,
                ]
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    format!("  {i}: {name}  ", i = i + 1)
                        .fg(SLATE.c200)
                        .bg(name.color())
                })
                .intersperse(Span::from(" "))
                .collect_vec(),
            )
            .centered(),
        )
        .wrap(Wrap { trim: false })
    }

    /// A bar like `<----- 80 px (gap: 2 px) ----->`
    ///
    /// Only shows the gap when spacing is not zero
    fn axis(&self, width: u16) -> impl Widget {
        let label = if self.spacing != 0 {
            format!("{} px (gap: {} px)", width, self.spacing)
        } else {
            format!("{} px", width)
        };
        let bar_width = width.saturating_sub(2) as usize; // we want to `<` and `>` at the ends
        let width_bar = format!("<{label:-^bar_width$}>");
        Paragraph::new(width_bar).fg(Self::AXIS_COLOR).centered()
    }

    fn render_layout_blocks(&self, area: Rect, buf: &mut Buffer) {
        let [user_constraints, area] =
            area.split(&Layout::vertical([Length(3), Fill(1)]).spacing(1));

        self.render_user_constraints_legend(user_constraints, buf);

        let [start, center, end, space_around, space_between] =
            area.split(&Layout::vertical([Length(7); 5]));

        self.render_layout_block(Flex::Start, start, buf);
        self.render_layout_block(Flex::Center, center, buf);
        self.render_layout_block(Flex::End, end, buf);
        self.render_layout_block(Flex::SpaceAround, space_around, buf);
        self.render_layout_block(Flex::SpaceBetween, space_between, buf)
    }

    fn render_user_constraints_legend(&self, area: Rect, buf: &mut Buffer) {
        let blocks = Layout::horizontal(
            self.constraints
                .iter()
                .map(|_| Constraint::Fill(1))
                .collect_vec(),
        )
        .split(area);

        for (i, (area, constraint)) in blocks.iter().zip(self.constraints.iter()).enumerate() {
            let selected = self.selected_index == i;
            ConstraintBlock::new(*constraint, selected, true).render(*area, buf);
        }
    }

    fn render_layout_block(&self, flex: Flex, area: Rect, buf: &mut Buffer) {
        let [label_area, axis_area, blocks_area] =
            area.split(&Layout::vertical([Length(1), Max(1), Length(4)]));

        if label_area.height > 0 {
            format!("Flex::{:?}", flex).bold().render(label_area, buf);
        }

        self.axis(area.width).render(axis_area, buf);

        let (blocks, spacers) = Layout::horizontal(&self.constraints)
            .flex(flex)
            .spacing(self.spacing)
            .split_with_spacers(blocks_area);

        for (i, (area, constraint)) in blocks.iter().zip(self.constraints.iter()).enumerate() {
            let selected = self.selected_index == i;
            ConstraintBlock::new(*constraint, selected, false).render(*area, buf);
        }

        for area in spacers.iter() {
            SpacerBlock.render(*area, buf);
        }
    }
}

impl Widget for ConstraintBlock {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match area.height {
            1 => self.render_1px(area, buf),
            2 => self.render_2px(area, buf),
            _ => self.render_4px(area, buf),
        }
    }
}

impl ConstraintBlock {
    const TEXT_COLOR: Color = SLATE.c200;

    fn new(constraint: Constraint, selected: bool, legend: bool) -> Self {
        Self {
            constraint,
            selected,
            legend,
        }
    }

    fn label(&self, width: u16) -> String {
        let long_width = format!("{} px", width);
        let short_width = format!("{}", width);
        // border takes up 2 columns
        let available_space = width.saturating_sub(2) as usize;
        let width_label = if long_width.len() < available_space {
            long_width
        } else if short_width.len() < available_space {
            short_width
        } else {
            "".to_string()
        };
        format!("{}\n{}", self.constraint, width_label)
    }

    fn render_1px(&self, area: Rect, buf: &mut Buffer) {
        let lighter_color = ConstraintName::from(self.constraint).lighter_color();
        let main_color = ConstraintName::from(self.constraint).color();
        let selected_color = if self.selected {
            lighter_color
        } else {
            main_color
        };
        Block::default()
            .fg(Self::TEXT_COLOR)
            .bg(selected_color)
            .render(area, buf);
    }

    fn render_2px(&self, area: Rect, buf: &mut Buffer) {
        let lighter_color = ConstraintName::from(self.constraint).lighter_color();
        let main_color = ConstraintName::from(self.constraint).color();
        let selected_color = if self.selected {
            lighter_color
        } else {
            main_color
        };
        Block::bordered()
            .border_set(symbols::border::QUADRANT_OUTSIDE)
            .border_style(Style::reset().fg(selected_color).reversed())
            .render(area, buf);
    }

    fn render_4px(&self, area: Rect, buf: &mut Buffer) {
        let lighter_color = ConstraintName::from(self.constraint).lighter_color();
        let main_color = ConstraintName::from(self.constraint).color();
        let selected_color = if self.selected {
            lighter_color
        } else {
            main_color
        };
        let color = if self.legend {
            selected_color
        } else {
            main_color
        };
        let label = self.label(area.width);
        let block = Block::bordered()
            .border_set(symbols::border::QUADRANT_OUTSIDE)
            .border_style(Style::reset().fg(color).reversed())
            .fg(Self::TEXT_COLOR)
            .bg(color);
        Paragraph::new(label)
            .centered()
            .fg(Self::TEXT_COLOR)
            .bg(color)
            .block(block)
            .render(area, buf);

        if !self.legend {
            let border_color = if self.selected {
                lighter_color
            } else {
                main_color
            };
            if let Some(last_row) = area.rows().last() {
                buf.set_style(last_row, border_color);
            }
        }
    }
}

impl Widget for SpacerBlock {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match area.height {
            1 => (),
            2 => self.render_2px(area, buf),
            3 => self.render_3px(area, buf),
            _ => self.render_4px(area, buf),
        }
    }
}

impl SpacerBlock {
    const TEXT_COLOR: Color = SLATE.c500;
    const BORDER_COLOR: Color = SLATE.c600;

    /// A block with a corner borders
    fn block() -> impl Widget {
        let corners_only = symbols::border::Set {
            top_left: line::NORMAL.top_left,
            top_right: line::NORMAL.top_right,
            bottom_left: line::NORMAL.bottom_left,
            bottom_right: line::NORMAL.bottom_right,
            vertical_left: " ",
            vertical_right: " ",
            horizontal_top: " ",
            horizontal_bottom: " ",
        };
        Block::bordered()
            .border_set(corners_only)
            .border_style(Self::BORDER_COLOR)
    }

    /// A vertical line used if there is not enough space to render the block
    fn line() -> impl Widget {
        Paragraph::new(Text::from(vec![
            Line::from(""),
            Line::from("│"),
            Line::from("│"),
            Line::from(""),
        ]))
        .style(Self::BORDER_COLOR)
    }

    /// A label that says "Spacer" if there is enough space
    fn spacer_label(width: u16) -> impl Widget {
        let label = if width >= 6 { "Spacer" } else { "" };
        label.fg(SpacerBlock::TEXT_COLOR).to_centered_line()
    }

    /// A label that says "8 px" if there is enough space
    fn label(width: u16) -> impl Widget {
        let long_label = format!("{width} px");
        let short_label = format!("{width}");
        let label = if long_label.len() < width as usize {
            long_label
        } else if short_label.len() < width as usize {
            short_label
        } else {
            "".to_string()
        };
        Line::styled(label, Self::TEXT_COLOR).centered()
    }

    fn render_2px(&self, area: Rect, buf: &mut Buffer) {
        if area.width > 1 {
            Self::block().render(area, buf);
        } else {
            Self::line().render(area, buf);
        }
    }

    fn render_3px(&self, area: Rect, buf: &mut Buffer) {
        if area.width > 1 {
            Self::block().render(area, buf);
        } else {
            Self::line().render(area, buf);
        }

        let row = area.rows().nth(1).unwrap_or_default();
        Self::spacer_label(area.width).render(row, buf);
    }

    fn render_4px(&self, area: Rect, buf: &mut Buffer) {
        if area.width > 1 {
            Self::block().render(area, buf);
        } else {
            Self::line().render(area, buf);
        }

        let row = area.rows().nth(1).unwrap_or_default();
        Self::spacer_label(area.width).render(row, buf);

        let row = area.rows().nth(2).unwrap_or_default();
        Self::label(area.width).render(row, buf);
    }
}

impl ConstraintName {
    fn color(&self) -> Color {
        match self {
            Self::Length => SLATE.c700,
            Self::Percentage => SLATE.c800,
            Self::Ratio => SLATE.c900,
            Self::Fill => SLATE.c950,
            Self::Min => BLUE.c800,
            Self::Max => BLUE.c900,
        }
    }

    fn lighter_color(&self) -> Color {
        match self {
            Self::Length => STONE.c500,
            Self::Percentage => STONE.c600,
            Self::Ratio => STONE.c700,
            Self::Fill => STONE.c800,
            Self::Min => SKY.c600,
            Self::Max => SKY.c700,
        }
    }
}

fn init_error_hooks() -> Result<()> {
    let (panic, error) = HookBuilder::default().into_hooks();
    let panic = panic.into_panic_hook();
    let error = error.into_eyre_hook();
    color_eyre::eyre::set_hook(Box::new(move |e| {
        let _ = restore_terminal();
        error(e)
    }))?;
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        panic(info)
    }));
    Ok(())
}

fn init_terminal() -> Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}