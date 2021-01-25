use toml;
use serde::Deserialize;
use std::fs;

// Baths are Deserialized from config file, nodes are a generated 3d graph on nodes from the 2d
// baths structs
#[derive(Deserialize, Debug)]
struct Bath {
    name: String,
    x: f32,
    y: f32,
    is_rinse: bool,
    neighbors: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Baths {
    bath: Vec<Bath>,
}

#[derive(Clone, Debug)]
pub struct Actions {
    gcode: String,
    seconds: f32,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub x: f32,
    pub y:f32,
    pub z: f32,
    pub is_rinse: bool,
    pub actions: Option<Vec<Actions>>,
    pub neighbors: Vec<String>,
}

#[derive(Clone)]
pub struct Nodes {
    pub node: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct NodeGrid2d {
    pub grid: Vec<Vec<Node>>,
}

impl Nodes {
    pub fn new() -> Nodes {
        Nodes {
            node: vec![]
        }
    }
    fn to_nodes(baths: Baths) -> Nodes {
        let mut nodes: Vec<Node> = vec![];
        let mut new_neighbors: Vec<String>;
        //let bath_iter = baths.bath.into_iter();
        for bath in baths.bath {
            // create node for head in bath
            
            nodes.push(
                Node {
                    name: format!("{}_inBath", bath.name),
                    x: bath.x,
                    y: bath.y,
                    z: -10.0,
                    is_rinse: bath.is_rinse,
                    actions: None,
                    neighbors: vec![bath.name.clone()]
                }
            );

            // create node for head above bath
            new_neighbors = bath.neighbors;
            new_neighbors.push(format!("{}_inBath", &bath.name));
            nodes.push(
                Node {
                    name: bath.name,
                    x: bath.x,
                    y: bath.y,
                    z: 0.0,
                    is_rinse : bath.is_rinse,
                    actions: None,
                    neighbors: new_neighbors,
                }
            )
        }
        Nodes {node: nodes}
    }

    
}

// A 2d vector grid split on the y axis where:
// vec![x x x x x x]
// vec![x x x x x x]
impl NodeGrid2d {
    fn new(grid: Vec<Vec<Node>>) -> NodeGrid2d {
        NodeGrid2d {
            grid,
        }
    }
    pub fn from_nodes(nodes: Nodes) -> NodeGrid2d {
        let mut node_vec = nodes.node.clone();
        // sort by y
        node_vec.retain(|n| n.z == 0.0);
        node_vec.sort_by(|a, b| (b.y).total_cmp(&a.y));
        // split into many y vecs
        let mut test_value: f32 = node_vec[0].y;
        let mut push_vec: usize = 0;
        let mut build_grid: Vec<Vec<Node>> = vec![Vec::new()];
        for node in node_vec {
            if (node.y - test_value).abs() < 2.0 {
                build_grid[push_vec].push(node);
            } else {
                push_vec += 1;
                test_value = node.y;
                build_grid.push(Vec::new());
                build_grid[push_vec].push(node);
            }

        }
        for i in 0..build_grid.len() {
            build_grid[i].sort_by(|a, b| (b.x).total_cmp(&a.x));
        }
        let ng2d = NodeGrid2d::new(build_grid);
        ng2d
    }
}

pub fn gen_nodes() -> Nodes {
    Nodes::to_nodes(get_baths_config())
}

fn get_baths_config() -> Baths {
    let baths_toml = &fs::read_to_string("config/baths.toml")
        .expect("Unable to open config/baths.toml");
    toml::from_str::<Baths>(baths_toml).unwrap()
}