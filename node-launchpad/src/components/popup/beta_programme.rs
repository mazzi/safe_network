// Copyright 2024 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::super::utils::centered_rect_fixed;
use super::super::Component;
use crate::{
    action::{Action, OptionsActions},
    mode::{InputMode, Scene},
    style::{clear_area, EUCALYPTUS, GHOST_WHITE, INDIGO, LIGHT_PERIWINKLE, RED, VIVID_SKY_BLUE},
    widgets::hyperlink::Hyperlink,
};
use color_eyre::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use regex::Regex;
use tui_input::{backend::crossterm::EventHandler, Input};

const INPUT_SIZE_USERNAME: u16 = 42; // Etherum address plus 0x
const INPUT_AREA_USERNAME: u16 = INPUT_SIZE_USERNAME + 2; // +2 for the padding

pub struct BetaProgramme {
    /// Whether the component is active right now, capturing keystrokes + draw things.
    active: bool,
    state: BetaProgrammeState,
    discord_input_field: Input,
    // cache the old value incase user presses Esc.
    old_value: String,
    back_to: Scene,
    can_save: bool,
}

#[allow(dead_code)]
enum BetaProgrammeState {
    DiscordIdAlreadySet,
    ShowTCs,
    RejectTCs,
    AcceptTCsAndEnterDiscordId,
}

impl BetaProgramme {
    pub fn new(username: String) -> Self {
        let state = if username.is_empty() {
            BetaProgrammeState::ShowTCs
        } else {
            BetaProgrammeState::DiscordIdAlreadySet
        };
        Self {
            active: false,
            state,
            discord_input_field: Input::default().with_value(username),
            old_value: Default::default(),
            back_to: Scene::Status,
            can_save: false,
        }
    }

    pub fn validate(&mut self) {
        if self.discord_input_field.value().is_empty() {
            self.can_save = false;
        } else {
            let re = Regex::new(r"^0x[a-fA-F0-9]{40}$").expect("Failed to compile regex");
            self.can_save = re.is_match(self.discord_input_field.value());
        }
    }

    fn capture_inputs(&mut self, key: KeyEvent) -> Vec<Action> {
        let send_back = match key.code {
            KeyCode::Enter => {
                self.validate();
                if self.can_save {
                    let username = self.discord_input_field.value().to_string().to_lowercase();
                    self.discord_input_field = username.clone().into();

                    debug!(
                        "Got Enter, saving the discord username {username:?}  and switching to DiscordIdAlreadySet, and Home Scene",
                    );
                    self.state = BetaProgrammeState::DiscordIdAlreadySet;
                    return vec![
                        Action::StoreDiscordUserName(username.clone()),
                        Action::OptionsActions(OptionsActions::UpdateBetaProgrammeUsername(
                            username,
                        )), // FIXME: Change OptionsActions::UpdateBetaProgrammeUsername name
                        Action::SwitchScene(Scene::Status),
                    ];
                }
                vec![]
            }
            KeyCode::Esc => {
                debug!(
                    "Got Esc, restoring the old value {} and switching to actual screen",
                    self.old_value
                );
                // reset to old value
                self.discord_input_field = self
                    .discord_input_field
                    .clone()
                    .with_value(self.old_value.clone());
                vec![Action::SwitchScene(self.back_to)]
            }
            KeyCode::Char(' ') => vec![],
            KeyCode::Backspace => {
                // if max limit reached, we should allow Backspace to work.
                self.discord_input_field.handle_event(&Event::Key(key));
                self.validate();
                vec![]
            }
            _ => {
                if self.discord_input_field.value().chars().count() < INPUT_SIZE_USERNAME as usize {
                    self.discord_input_field.handle_event(&Event::Key(key));
                    self.validate();
                }
                vec![]
            }
        };
        send_back
    }
}

impl Component for BetaProgramme {
    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Vec<Action>> {
        if !self.active {
            return Ok(vec![]);
        }
        // while in entry mode, keybinds are not captured, so gotta exit entry mode from here
        let send_back = match &self.state {
            BetaProgrammeState::DiscordIdAlreadySet => self.capture_inputs(key),
            BetaProgrammeState::ShowTCs => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let is_discord_id_set = !self.discord_input_field.value().is_empty();
                    if is_discord_id_set {
                        debug!("User accepted the TCs, but discord id already set, moving to DiscordIdAlreadySet");
                        self.state = BetaProgrammeState::DiscordIdAlreadySet;
                    } else {
                        debug!("User accepted the TCs, but no discord id set, moving to AcceptTCsAndEnterDiscordId");
                        self.state = BetaProgrammeState::AcceptTCsAndEnterDiscordId;
                    }
                    vec![]
                }
                KeyCode::Esc => {
                    debug!("User rejected the TCs, moving to original screen");
                    self.state = BetaProgrammeState::ShowTCs;
                    vec![Action::SwitchScene(self.back_to)]
                }
                _ => {
                    vec![]
                }
            },
            BetaProgrammeState::RejectTCs => {
                if let KeyCode::Esc = key.code {
                    debug!("RejectTCs msg closed. Switching to Status scene.");
                    self.state = BetaProgrammeState::ShowTCs;
                }
                vec![Action::SwitchScene(self.back_to)]
            }
            BetaProgrammeState::AcceptTCsAndEnterDiscordId => self.capture_inputs(key),
        };
        Ok(send_back)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let send_back = match action {
            Action::SwitchScene(scene) => match scene {
                Scene::StatusBetaProgrammePopUp | Scene::OptionsBetaProgrammePopUp => {
                    self.active = true;
                    self.old_value = self.discord_input_field.value().to_string();
                    if scene == Scene::StatusBetaProgrammePopUp {
                        self.back_to = Scene::Status;
                    } else if scene == Scene::OptionsBetaProgrammePopUp {
                        self.back_to = Scene::Options;
                    }
                    // Set to InputMode::Entry as we want to handle everything within our handle_key_events
                    // so by default if this scene is active, we capture inputs.
                    Some(Action::SwitchInputMode(InputMode::Entry))
                }
                _ => {
                    self.active = false;
                    None
                }
            },
            _ => None,
        };
        Ok(send_back)
    }

    fn draw(&mut self, f: &mut crate::tui::Frame<'_>, area: Rect) -> Result<()> {
        if !self.active {
            return Ok(());
        }

        let layer_zero = centered_rect_fixed(52, 15, area);

        let layer_one = Layout::new(
            Direction::Vertical,
            [
                // for the pop_up_border
                Constraint::Length(2),
                // for the input field
                Constraint::Min(1),
                // for the pop_up_border
                Constraint::Length(1),
            ],
        )
        .split(layer_zero);

        // layer zero
        let pop_up_border = Paragraph::new("").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Add Your Wallet ")
                .bold()
                .title_style(Style::new().fg(VIVID_SKY_BLUE))
                .padding(Padding::uniform(2))
                .border_style(Style::new().fg(VIVID_SKY_BLUE)),
        );
        clear_area(f, layer_zero);

        match self.state {
            BetaProgrammeState::DiscordIdAlreadySet => {
                self.validate(); // FIXME: maybe this should be somewhere else
                                 // split into 4 parts, for the prompt, input, text, dash , and buttons
                let layer_two = Layout::new(
                    Direction::Vertical,
                    [
                        // for the prompt text
                        Constraint::Length(3),
                        // for the input
                        Constraint::Length(1),
                        // for the text
                        Constraint::Length(6),
                        // gap
                        Constraint::Length(1),
                        // for the buttons
                        Constraint::Length(1),
                    ],
                )
                .split(layer_one[1]);

                let prompt_text = Paragraph::new(Line::from(vec![
                    Span::styled("Enter new ".to_string(), Style::default()),
                    Span::styled("Wallet Address".to_string(), Style::default().bold()),
                ]))
                .block(Block::default())
                .alignment(Alignment::Center)
                .fg(GHOST_WHITE);

                f.render_widget(prompt_text, layer_two[0]);

                let spaces = " ".repeat(
                    (INPUT_AREA_USERNAME - 1) as usize - self.discord_input_field.value().len(),
                );
                let input = Paragraph::new(Span::styled(
                    format!("{}{} ", spaces, self.discord_input_field.value()),
                    Style::default()
                        .fg(if self.can_save { VIVID_SKY_BLUE } else { RED })
                        .bg(INDIGO)
                        .underlined(),
                ))
                .alignment(Alignment::Center);
                f.render_widget(input, layer_two[1]);

                let text = Paragraph::new(Text::from(if self.can_save {
                    vec![
                        Line::raw("Changing your Wallet will reset and restart"),
                        Line::raw("all your nodes."),
                    ]
                } else {
                    vec![Line::from(Span::styled(
                        "Invalid wallet address".to_string(),
                        Style::default().fg(RED),
                    ))]
                }))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .padding(Padding::horizontal(2))
                        .padding(Padding::top(2)),
                );

                f.render_widget(text.fg(GHOST_WHITE), layer_two[2]);

                let dash = Block::new()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::new().fg(GHOST_WHITE));
                f.render_widget(dash, layer_two[3]);

                let buttons_layer = Layout::horizontal(vec![
                    Constraint::Percentage(55),
                    Constraint::Percentage(45),
                ])
                .split(layer_two[4]);

                let button_no = Line::from(vec![Span::styled(
                    "  Cancel [Esc]",
                    Style::default().fg(LIGHT_PERIWINKLE),
                )]);

                f.render_widget(button_no, buttons_layer[0]);

                let button_yes = Line::from(vec![Span::styled(
                    "Change Wallet [Enter]",
                    if self.can_save {
                        Style::default().fg(EUCALYPTUS)
                    } else {
                        Style::default().fg(LIGHT_PERIWINKLE)
                    },
                )]);
                f.render_widget(button_yes, buttons_layer[1]);
            }
            BetaProgrammeState::ShowTCs => {
                // split the area into 3 parts, for the lines, hypertext,  buttons
                let layer_two = Layout::new(
                    Direction::Vertical,
                    [
                        // for the text
                        Constraint::Length(7),
                        // for the hypertext
                        Constraint::Length(1),
                        // gap
                        Constraint::Length(5),
                        // for the buttons
                        Constraint::Length(1),
                    ],
                )
                .split(layer_one[1]);

                let text = Paragraph::new(vec![
                    Line::from(Span::styled("Add your wallet and you can earn a slice of millions of tokens created at the genesis of the Autonomi Network when through running nodes.",Style::default())),
                    Line::from(Span::styled("\n\n",Style::default())),
                    Line::from(Span::styled("By continuing you agree to the Terms and Conditions found here:",Style::default())),
                    Line::from(Span::styled("\n\n",Style::default())),
                    ]
                )
                .block(Block::default().padding(Padding::horizontal(2)))
                .wrap(Wrap { trim: false });

                f.render_widget(text.fg(GHOST_WHITE), layer_two[0]);

                let link = Hyperlink::new(
                    Span::styled(
                        "  https://autonomi.com/beta/terms",
                        Style::default().fg(VIVID_SKY_BLUE),
                    ),
                    "https://autonomi.com/beta/terms",
                );

                f.render_widget_ref(link, layer_two[1]);

                let dash = Block::new()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::new().fg(GHOST_WHITE));
                f.render_widget(dash, layer_two[2]);

                let buttons_layer = Layout::horizontal(vec![
                    Constraint::Percentage(45),
                    Constraint::Percentage(55),
                ])
                .split(layer_two[3]);

                let button_no = Line::from(vec![Span::styled(
                    "  No, Cancel [Esc]",
                    Style::default().fg(LIGHT_PERIWINKLE),
                )]);
                f.render_widget(button_no, buttons_layer[0]);

                let button_yes = Paragraph::new(Line::from(vec![Span::styled(
                    "Yes, I agree! Continue [Y]  ",
                    Style::default().fg(EUCALYPTUS),
                )]))
                .alignment(Alignment::Right);
                f.render_widget(button_yes, buttons_layer[1]);
            }
            BetaProgrammeState::RejectTCs => {
                // split the area into 3 parts, for the lines, hypertext,  buttons
                let layer_two = Layout::new(
                    Direction::Vertical,
                    [
                        // for the text
                        Constraint::Length(7),
                        // gap
                        Constraint::Length(5),
                        // for the buttons
                        Constraint::Length(1),
                    ],
                )
                .split(layer_one[1]);

                let text = Paragraph::new(vec![
                    Line::from(Span::styled("Terms and conditions not accepted.",Style::default())),
                    Line::from(Span::styled("\n\n",Style::default())),
                    Line::from(Span::styled("Beta Rewards Program entry not approved.",Style::default())),
                    Line::from(Span::styled("\n\n",Style::default())),
                    Line::from(Span::styled("You can still run nodes on the network, but you will not be part of the Beta Rewards Program.",Style::default())),
                    ]
                )
                .block(Block::default().padding(Padding::horizontal(2)))
                .wrap(Wrap { trim: false });

                f.render_widget(text.fg(GHOST_WHITE), layer_two[0]);

                let dash = Block::new()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::new().fg(GHOST_WHITE));
                f.render_widget(dash, layer_two[1]);
                let line = Line::from(vec![Span::styled(
                    "  Close [Esc]",
                    Style::default().fg(LIGHT_PERIWINKLE),
                )]);
                f.render_widget(line, layer_two[2]);
            }
            BetaProgrammeState::AcceptTCsAndEnterDiscordId => {
                // split into 4 parts, for the prompt, input, text, dash , and buttons
                let layer_two = Layout::new(
                    Direction::Vertical,
                    [
                        // for the prompt text
                        Constraint::Length(3),
                        // for the input
                        Constraint::Length(2),
                        // for the text
                        Constraint::Length(3),
                        // for the hyperlink
                        Constraint::Length(2),
                        // gap
                        Constraint::Length(1),
                        // for the buttons
                        Constraint::Length(1),
                    ],
                )
                .split(layer_one[1]);

                let prompt = Paragraph::new(Line::from(vec![
                    Span::styled("Enter your ", Style::default()),
                    Span::styled("Wallet Address", Style::default().fg(GHOST_WHITE)),
                ]))
                .alignment(Alignment::Center);

                f.render_widget(prompt.fg(GHOST_WHITE), layer_two[0]);

                let spaces = " ".repeat(
                    (INPUT_AREA_USERNAME - 1) as usize - self.discord_input_field.value().len(),
                );
                let input = Paragraph::new(Span::styled(
                    format!("{}{} ", spaces, self.discord_input_field.value()),
                    Style::default().fg(VIVID_SKY_BLUE).bg(INDIGO).underlined(),
                ))
                .alignment(Alignment::Center);
                f.render_widget(input, layer_two[1]);

                let text = Paragraph::new(vec![Line::from(Span::styled(
                    "Find out more about compatible wallets, and how to track your earnings:",
                    Style::default(),
                ))])
                .block(Block::default().padding(Padding::horizontal(2)))
                .wrap(Wrap { trim: false });

                f.render_widget(text.fg(GHOST_WHITE), layer_two[2]);

                let link = Hyperlink::new(
                    Span::styled(
                        "  https://autonomi.com/wallet",
                        Style::default().fg(VIVID_SKY_BLUE),
                    ),
                    "https://autonomi.com/wallet",
                );

                f.render_widget_ref(link, layer_two[3]);

                let dash = Block::new()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::new().fg(GHOST_WHITE));
                f.render_widget(dash, layer_two[4]);

                let buttons_layer = Layout::horizontal(vec![
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(layer_two[5]);

                let button_no = Line::from(vec![Span::styled(
                    "  Cancel [Esc]",
                    Style::default().fg(LIGHT_PERIWINKLE),
                )]);
                f.render_widget(button_no, buttons_layer[0]);
                let button_yes = Paragraph::new(Line::from(vec![Span::styled(
                    "Save Wallet [Enter]  ",
                    if self.can_save {
                        Style::default().fg(EUCALYPTUS)
                    } else {
                        Style::default().fg(LIGHT_PERIWINKLE)
                    },
                )]))
                .alignment(Alignment::Right);
                f.render_widget(button_yes, buttons_layer[1]);
            }
        }

        f.render_widget(pop_up_border, layer_zero);

        Ok(())
    }
}
