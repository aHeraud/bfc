extern crate lalrpop;

fn main() {
    // this rebuilds the parser from our lalrpop grammar file at build time
    lalrpop::process_root().unwrap();
}