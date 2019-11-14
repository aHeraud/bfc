use target_lexicon::Triple;

use cranelift::prelude::{EntityRef, InstBuilder};
use cranelift::codegen::ir;
use cranelift::codegen::ir::{AbiParam, ExternalName, FuncRef};
use cranelift::codegen::Context;

use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext, Variable};

use cranelift_module::{Module, Backend, Linkage, FuncId};

use crate::ast::Node;

pub struct CodegenContext<B: Backend> {
    module: Module<B>,
    context: Context,
    func_context: FunctionBuilderContext,
    cell_type: ir::Type,
    pointer_type: ir::Type,
    cell_count: usize
}

impl<B: Backend> CodegenContext<B> {
    pub fn new(module: Module<B>, target_triple: Triple, cell_type: ir::Type, cell_count: usize) -> CodegenContext<B> {
        let context = module.make_context();
        CodegenContext {
            module,
            context,
            func_context: FunctionBuilderContext::new(),
            cell_type,
            pointer_type: ir::Type::triple_pointer_type(&target_triple),
            cell_count
        }
    }

    pub fn finish(self) -> B::Product {
        self.module.finish()
    }

    pub fn generate(&mut self, program: Vec<Node>) {
        let libc_calloc_id = self.declare_libc_calloc().unwrap();
        let libc_putchar_id = self.declare_libc_putchar().unwrap();
        let libc_getchar_id = self.declare_libc_getchar().unwrap();

        let mut signature = self.module.make_signature();
        signature.returns.push(AbiParam::new(ir::types::I32));
        let function_id = self.module.declare_function("main", Linkage::Export, &signature).unwrap();
        self.context.func.signature = signature;
        self.context.func.name = ExternalName::user(0, function_id.as_u32());
        {
            let mut builder = FunctionBuilder::new(&mut self.context.func, &mut self.func_context);
            let entry_block = builder.create_ebb();

            let pointer = Variable::new(0);
            builder.declare_var(pointer, self.pointer_type);

            builder.append_ebb_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            let libc_calloc = self.module.declare_func_in_func(libc_calloc_id, &mut builder.func);
            let libc_putchar = self.module.declare_func_in_func(libc_putchar_id, &mut builder.func);
            let libc_getchar = self.module.declare_func_in_func(libc_getchar_id, &mut builder.func);

            {
                // allocate
                let count = builder.ins().iconst(ir::types::I64, self.cell_count as i64);
                let size = builder.ins().iconst(ir::types::I64, self.cell_type.bytes() as i64);
                let val = ir::entities::Value::from_u32(builder.ins().call(libc_calloc, &[count, size]).as_u32());
                builder.def_var(pointer, val);
            }

            {
                let mut ctx = TranslationContext::new(&mut builder, self.pointer_type, 
                    self.cell_type, entry_block, pointer, libc_putchar, libc_getchar);
                for node in program {
                    node.emit(&mut ctx);
                }
                //ctx.builder.seal_block(ctx.block);
            }

            let ret_val = builder.ins().iconst(ir::types::I32, 0);
            builder.ins().return_(&[ret_val]);

            builder.seal_all_blocks();
            builder.finalize();
        }

        self.module.define_function(function_id, &mut self.context).expect("Failed to define function");
        self.module.clear_context(&mut self.context);
        self.module.finalize_definitions();
    }

    fn declare_libc_putchar(&mut self) -> cranelift_module::ModuleResult<FuncId> {
        let mut signature = self.module.make_signature();
        signature.params.push(AbiParam::new(ir::types::I8));
        signature.returns.push(AbiParam::new(ir::types::I32));
        self.module.declare_function("putchar", Linkage::Import, &signature)
    }

    fn declare_libc_calloc(&mut self) -> cranelift_module::ModuleResult<FuncId> {
        let mut signature = self.module.make_signature();
        signature.params.push(AbiParam::new(ir::types::I64));
        signature.params.push(AbiParam::new(ir::types::I64));
        signature.returns.push(AbiParam::new(ir::types::I64));
        self.module.declare_function("calloc", Linkage::Import, &signature)
    }

    fn declare_libc_getchar(&mut self) -> cranelift_module::ModuleResult<FuncId> {
        let mut signature = self.module.make_signature();
        signature.returns.push(AbiParam::new(ir::types::I32));
        self.module.declare_function("getchar", Linkage::Import, &signature)
    }
}

struct TranslationContext<'a, 'b> {
    builder: &'b mut FunctionBuilder<'a>,
    pointer_type: ir::Type,
    cell_type: ir::Type,
    block: cranelift::prelude::Ebb,
    pointer: Variable,
    putchar: FuncRef,
    getchar: FuncRef
}

impl<'a, 'b> TranslationContext<'a, 'b> {
    pub fn new(builder: &'b mut FunctionBuilder<'a>, pointer_type: ir::Type, cell_type: ir::Type, block: cranelift::prelude::Ebb, pointer: Variable, putchar: FuncRef, getchar: FuncRef) -> TranslationContext<'a, 'b> {
        TranslationContext {
            builder,
            pointer_type,
            cell_type,
            block,
            pointer,
            putchar,
            getchar
        }
    }
}

trait Translate {
    fn emit<'a, 'b>(&self, ctx: &mut TranslationContext<'a, 'b>);
}

impl Translate for Node {
    fn emit<'a, 'b>(&self, ctx: &mut TranslationContext<'a, 'b>) {
        use cranelift::codegen::ir::MemFlags;
        match self {
            Node::IncrementPointer => {
                let offset = ctx.builder.ins().iconst(ctx.pointer_type, ctx.cell_type.bytes() as i64);
                let arg1 = ctx.builder.use_var(ctx.pointer);
                let result = ctx.builder.ins().iadd(arg1, offset);
                ctx.builder.def_var(ctx.pointer, result);
            },
            Node::DecrementPointer => {
                let offset = ctx.builder.ins().iconst(ctx.pointer_type, ctx.cell_type.bytes() as i64);
                let arg1 = ctx.builder.use_var(ctx.pointer);
                let result = ctx.builder.ins().isub(arg1, offset);
                ctx.builder.def_var(ctx.pointer, result);
            },
            Node::Increment => {
                let address = ctx.builder.use_var(ctx.pointer);
                let data = ctx.builder.ins().load(ctx.cell_type, MemFlags::new(), address, 0);
                let const_1 = ctx.builder.ins().iconst(ctx.cell_type, 1);
                let result = ctx.builder.ins().iadd(data, const_1);
                ctx.builder.ins().store(MemFlags::new(), result, address, 0);
            },
            Node::Decrement => {
                let address = ctx.builder.use_var(ctx.pointer);
                let data = ctx.builder.ins().load(ctx.cell_type, MemFlags::new(), address, 0);
                let const_1 = ctx.builder.ins().iconst(ctx.cell_type, 1);
                let result = ctx.builder.ins().isub(data, const_1);
                ctx.builder.ins().store(MemFlags::new(), result, address, 0);
            },
            Node::Loop(body) => {
                //ctx.builder.seal_block(ctx.block);

                let initial_body_block = ctx.builder.create_ebb();
                let next_block = ctx.builder.create_ebb();

                // the opening loop bracket, '[', means to jump past the closing bracket (']')
                // if the cell under the pointer is zero.

                // first load the value in the cell under the pointer
                let address = ctx.builder.use_var(ctx.pointer);
                let data = ctx.builder.ins().load(ctx.cell_type, MemFlags::new(), address, 0);

                // if data is zero, then we jump past the loop body
                ctx.builder.ins().brz(data, next_block, &[]); // FIXME: missing encoding

                // otherwise we fallthrough to the next ebb, which will be filled by the loop body
                ctx.builder.ins().fallthrough(initial_body_block, &[]);
                ctx.builder.switch_to_block(initial_body_block);
                ctx.block = initial_body_block;

                for node in body {
                    node.emit(ctx);
                }

                // loop backedge:
                // the closing loop bracket, ']', jumps back to the instruction after the matching opening
                // loop bracket ('[') if the cell under the pointer is not zero.

                // load the value in the cell under the pointer
                let address = ctx.builder.use_var(ctx.pointer);
                let data = ctx.builder.ins().load(ctx.cell_type, MemFlags::new(), address, 0);

                // jump back to the beginning of the loop body if data is not zero
                ctx.builder.ins().brnz(data, initial_body_block, &[]); //FIXME: missing encoding

                // otherwise fallthrough to the next ebb, which is past the end of the loop
                ctx.builder.ins().fallthrough(next_block, &[]);
                ctx.builder.switch_to_block(next_block);
                ctx.block = next_block;

                // we've reached the end of the loop, and have already seen all
                // backedges to the initial body block
                //ctx.builder.seal_block(initial_body_block);
            },
            Node::PrintChar => {
                // prints the character for the value currently under the pointer
                // uses libc's putchar function

                // read the value under the pointer
                let address = ctx.builder.use_var(ctx.pointer);
                let data = ctx.builder.ins().load(ctx.cell_type, MemFlags::new(), address, 0);

                let value = if ctx.cell_type != ir::types::I8 {
                    // convert to a char
                    ctx.builder.ins().ireduce(ir::types::I8, data)
                }
                else {
                    data
                };

                ctx.builder.ins().call(ctx.putchar, &[value]);
            },
            _ => panic!(format!("Codegen for {:?} not implemented", self))
        };
    }
}
