use trellis_core::{Graph, InputNode};

#[derive(Clone, PartialEq)]
struct ProjectId(u64);

#[derive(Clone, PartialEq)]
struct UserId(u64);

fn needs_user(_: InputNode<UserId>) {}

fn main() {
    let mut graph = Graph::new();
    let mut tx = graph.begin_transaction().unwrap();
    let project = tx.input::<ProjectId>("project").unwrap();

    needs_user(project);
}
