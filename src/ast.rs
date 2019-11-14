#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Node {
    IncrementPointer,
    DecrementPointer,
    Increment,
    Decrement,
    PrintChar,
    ReadChar,
    Loop(Vec<Node>)
}
