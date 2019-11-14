#[macro_use] extern crate lalrpop_util;
extern crate structopt;
extern crate cranelift;
extern crate cranelift_module;
extern crate cranelift_faerie;
#[macro_use] extern crate target_lexicon;

use structopt::StructOpt;

mod ast;
mod lexer;
mod codegen;
mod options;
lalrpop_mod!(grammar);

use options::Options;

fn main() {
    use std::fs;
    use std::fs::File;
    use std::str::FromStr;
    use cranelift_faerie::FaerieBuilder;
    use cranelift_module::Module;
    use cranelift::codegen::ir;
    use cranelift::codegen::settings::{Flags, Configurable};
    use lexer::Lexer;

    let args = Options::from_args();

    let source = fs::read_to_string(args.source).unwrap();
    let lexer = Lexer::new(&source);

    let ast = grammar::ProgramParser::new()
        .parse(lexer)
        .unwrap();

    //let triple = target_lexicon::HOST.clone();
    let triple = triple!("x86_64-unknown-linux-elf");

    let flags = {
        let mut flag_builder = cranelift::codegen::settings::builder();
        flag_builder.enable("is_pic").unwrap();
        Flags::new(flag_builder)
    };

    let isa = cranelift::codegen::isa::lookup(triple.clone())
        .expect("target tripple unsupported")
        .finish(flags.clone());

    let backend_builder = FaerieBuilder::new(isa,
        String::from("program"),
        cranelift_faerie::FaerieTrapCollection::Disabled,
        cranelift_module::default_libcall_names()).expect("failed to create backend builder");

    let cell_type: ir::types::Type = ir::types::Type::int(args.cell_size.bytes() as u16 * 8).unwrap();

    let mut context: codegen::CodegenContext<cranelift_faerie::FaerieBackend> = codegen::CodegenContext::new(Module::new(backend_builder), triple, cell_type, args.memory_size);
    context.generate(ast);

    let result = context.finish();
    let output_file = File::create("out.o").expect("Failed to create output file.");
    result.write(output_file).expect("Failed to write object file to output");
}
