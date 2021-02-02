#![feature(total_cmp)]
mod nodes;
mod grbl;
mod paths;
use nodes::{Node, Nodes, NodeGrid2d};
use grbl::Grbl;
use std::collections::HashMap;
use regex::Regex;

use iced::{button, Button, time, Scrollable, Subscription, Space, Font, Checkbox, scrollable, Container, Command, HorizontalAlignment, Length ,Column, Row, Element, Application, Settings, Text};

pub fn main() -> iced::Result {
    Bathtub::run(Settings::default())
}

//#[derive(Debug)]
enum Bathtub {
    Loading,
    Loaded(State)
}

struct State {
    scroll: scrollable::State,
    bath_btns: Vec<Vec<Option<(Node, button::State)>>>,
    title: String,
    nodes: Nodes,
    node_map: HashMap<String, usize>,
    current_node: Node,
    in_bath: bool,
    grbl: Grbl,
    status_regex: Regex,
    connected: bool,
}


#[derive(Debug, Clone)]
struct LoadState {
    nodes: Nodes,
    node_map: HashMap<String, usize>,
    node_grid2d: NodeGrid2d,
}

#[derive(Debug, Clone)]
enum LoadError {
    _Placeholder,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<LoadState, LoadError>),
    ButtonPressed(String),
    ToggleBath(bool),
    Tick,
}

impl Application for Bathtub {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Bathtub, Command<Message>) {
        (
            Bathtub::Loading,
            Command::perform(LoadState::load(), Message::Loaded),
        )
    }

    fn title(&self) -> String {
        String::from("Bathtub")
    }

    fn subscription(&self) -> Subscription<Message> {
        match self {
            Bathtub::Loaded(state) => {
                if state.connected {
                    return time::every(std::time::Duration::from_millis(50))
                        .map(|_| Message::Tick)
                } else {
                    return Subscription::none();
                }
            },
            _ => Subscription::none(),
        }
    }
    
    fn update(&mut self, message: Message) -> Command<Message> {
        match self {
            Bathtub::Loading => {
                match message {
                    Message::Loaded(Ok(state)) => {
                        *self = Bathtub::Loaded(State {
                            bath_btns: state.node_grid2d.grid.into_iter()
                                .fold(Vec::new(), |mut vec, axis| {
                                    vec.push(
                                        axis.into_iter()
                                            // this might not work
                                            .fold(Vec::new(), |mut axis_vec, node| {
                                                println!("{:?}", node);
                                                if let Some(n) = node {
                                                    axis_vec.push(Some((n, button::State::new())));
                                                } else {
                                                    axis_vec.push(None);
                                                }
                                                axis_vec
                                            })
                                    );
                                    vec
                                }),
                            scroll: scrollable::State::new(),
                            title: "Click any button\nto start homing cycle".to_string(),
                            nodes: state.nodes.clone(),
                            node_map: state.node_map.clone(),
                            current_node: state.nodes.node[state.node_map.get(&"MCL_16".to_string()).unwrap().clone()].clone(),
                            in_bath: false,
                            connected: false,
                            grbl: grbl::new(),
                            status_regex: Regex::new(r"(?P<status>[A-Za-z]+).{6}(?P<X>[-\d.]+),(?P<Y>[-\d.]+),(?P<Z>[-\d.]+)").unwrap(),

                        });
                    }
                    Message::Loaded(Err(_)) => {
                        panic!("somehow loaded had an error")
                        // need to add correct fail state, following is from the Todos example
                        //*self = Bathtub::Loaded(State::default());
                    }
                    Message::ButtonPressed(btn_name) => {
                        // This is not used, might not be necessary
                        println!("{} was pressed", btn_name);
                    }
                    Message::ToggleBath(_bool) => {
                        //this is not used
                        ()
                    }
                    Message::Tick => {
                        // this is not used
                    }
                }
                Command::none()
            }
            Bathtub::Loaded(state) => {
                match message {
                    Message::ButtonPressed(btn) => {
                        // requires homing first
                        if !state.connected {
                            state.title = "Running Homing Cycle\nPlease wait".to_string();
                            state.grbl.send("$H".to_string()).unwrap();
                            state.connected = true;
                        }
                        // create path & gcode commands to be send
                        let enter_bath: String;
                        if state.in_bath {enter_bath = "_inBath".to_string();} else {enter_bath = "".to_string();}
                        let next_node = &state.nodes.node[state.node_map.get(&format!("{}{}",btn.clone(), enter_bath)).unwrap().clone()];
                        let node_paths = paths::gen_node_paths(&state.nodes, &state.current_node, next_node);
                        let gcode_paths = paths::gen_gcode_paths(&node_paths);
                        // send gcode
                        for gcode_path in gcode_paths {
                            state.grbl.send(gcode_path).unwrap();
                        }
                        state.current_node = next_node.clone();
                    },
                    Message::ToggleBath(boolean) => {
                        state.in_bath = boolean;
                    },
                    Message::Tick => {
                        state.grbl.send("?".to_string()).unwrap();
                        for _i in 0..3 {
                            match state.grbl.try_recv() {
                                Ok((_,cmd, msg)) if cmd == "?".to_string() => {
                                    if let Some(caps) = state.status_regex.captures(&msg[..]) {
                                        state.title = format!(
                                            "{} state at\n({:.3}, {:.3}, {:.3})",
                                            &caps["status"],
                                            &caps["X"].parse::<f32>().unwrap() + 1.0, //adjust to match Gcode inputs
                                            &caps["Y"].parse::<f32>().unwrap() + 1.0,
                                            &caps["Z"].parse::<f32>().unwrap() + 1.0
                                        );
                                    }
                                }
                                // do nothing for now. Will likely create a log of gcode commands
                                // in future
                                _ => {()}
                            }
                        }
                    }
                    _ => (),
                }
                Command::none()
            }
        }
    }
    
    fn view(&mut self) -> Element<Message> {
        match self {
            Bathtub::Loading => loading_message(),
            Bathtub::Loaded(State {
                                scroll,
                                bath_btns,
                                in_bath,
                                title,
                                ..
            }) => {
                let title = Text::new(title.clone())
                    .width(Length::Fill)
                    .size(40)
                    .color([0.5, 0.5, 0.5])
                    .font(MONOSPACE_TYPEWRITTER)
                    .horizontal_alignment(HorizontalAlignment::Center); 

                //println!("{:?}", bath_btns);
                let button_grid = bath_btns.into_iter()
                    .fold(Column::new(), |column, grid| {
                        column.push(grid.into_iter()
                            .fold(Row::new(), |row, node_tup| {
                                if let Some(nt) = node_tup {
                                    row.push(
                                        Button::new(&mut nt.1, Text::new(&nt.0.name).horizontal_alignment(HorizontalAlignment::Center))
                                            .padding(15)
                                            .width(Length::Fill)
                                            .on_press(Message::ButtonPressed(nt.0.name.clone()))
                                    )
                                } else {
                                    row.push(Column::new()
                                            .width(Length::Fill)
                                    )
                                }
                            }).padding(3)
                        )
                    });

                let inbath_toggle = Checkbox::new(
                  in_bath.clone(),
                  "Enter Bath",
                  Message::ToggleBath,
                );
                let content = Column::new()
                    .max_width(800)
                    .spacing(20)
                    .push(title)
                    .push(button_grid)
                    .push(inbath_toggle);

                Scrollable::new(scroll)
                    .padding(40)
                    .push(
                        Container::new(content).width(Length::Fill).center_x(),
                    )
                    .into()
            }
        }
    }
}

impl LoadState {
    fn new(nodes: Nodes, node_map: HashMap<String, usize>, node_grid2d: NodeGrid2d) -> LoadState {
        LoadState {
            nodes,
            node_map,
            node_grid2d,
        }
    }

    // This is just a placeholder. Will eventually read data from server
    async fn load() -> Result<LoadState, LoadError> {
        let nodes = nodes::gen_nodes();
        
        Ok(
            LoadState::new(nodes.clone(),
                nodes::get_nodemap(nodes.clone()),
                NodeGrid2d::from_nodes(nodes)
            )
        )
    }}

fn loading_message<'a>() -> Element<'a, Message> {
    Container::new(
            Text::new("Loading...")
                .horizontal_alignment(HorizontalAlignment::Center)
                .size(50),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_y()
    .into()
}

const MONOSPACE_TYPEWRITTER: Font = Font::External {
    name: "MonospaceTypewritter",
    bytes: include_bytes!("../fonts/MonospaceTypewriter.ttf"),
};

// Ideas
// Things that should be in the config file
// 1. auto find usb port, or error if not available
// 2. All usb settings should come from the config file
