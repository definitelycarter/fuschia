use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
  pub id: String,
  pub actor: String,
  #[serde(default)]
  pub config: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
  pub from: String,
  pub to: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Graph {
  pub entry: String,
  pub nodes: Vec<Node>,
  pub edges: Vec<Edge>,
}

impl Graph {
  pub fn edges_from<'a>(&'a self, node_id: &'a str) -> impl Iterator<Item = &'a Edge> + 'a {
    self.edges.iter().filter(move |e| e.from == node_id)
  }
}
