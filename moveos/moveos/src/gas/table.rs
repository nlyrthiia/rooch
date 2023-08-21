// Copyright (c) RoochNetwork
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::gas_algebra::{AbstractMemorySize, InternalGas, NumArgs, NumBytes};
use move_core_types::language_storage::ModuleId;
use move_core_types::vm_status::StatusCode;
use move_vm_types::gas::{GasMeter, SimpleInstruction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::views::{TypeView, ValueView};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The size in bytes for a reference on the stack
pub const REFERENCE_SIZE: AbstractMemorySize = AbstractMemorySize::new(8);

/// The size of a struct in bytes
pub const STRUCT_SIZE: AbstractMemorySize = AbstractMemorySize::new(2);

/// The size of a vector (without its containing data) in bytes
pub const VEC_SIZE: AbstractMemorySize = AbstractMemorySize::new(8);

#[derive(Clone, Debug, Serialize, PartialEq, Eq, Deserialize)]
pub struct CostTable {
    pub instruction_tiers: BTreeMap<u64, u64>,
    pub stack_height_tiers: BTreeMap<u64, u64>,
    pub stack_size_tiers: BTreeMap<u64, u64>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MoveOSGasMeter {
    pub gas_model_version: u64,
    cost_table: CostTable,
    gas_left: InternalGas,
    gas_price: u64,
    initial_budget: InternalGas,
    charge: bool,
}

impl Default for MoveOSGasMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveOSGasMeter {
    pub fn new() -> Self {
        Self {
            gas_model_version: 0,
            cost_table: CostTable {
                instruction_tiers: Default::default(),
                stack_height_tiers: Default::default(),
                stack_size_tiers: Default::default(),
            },
            gas_left: InternalGas::zero(),
            gas_price: 0,
            initial_budget: InternalGas::new(10000000),
            charge: false,
        }
    }

    pub fn charge(
        &mut self,
        _num_instructions: u64,
        _pushes: u64,
        _pops: u64,
        _incr_size: u64,
        _decr_size: u64,
    ) -> PartialVMResult<()> {
        // #TODO: Various resources are used to charge for the execution of an instruction.
        Ok(())
    }

    pub fn deduct_gas(&mut self, amount: InternalGas) -> PartialVMResult<()> {
        if !self.charge {
            return Ok(());
        }

        match self.gas_left.checked_sub(amount) {
            Some(gas_left) => {
                self.gas_left = gas_left;
                Ok(())
            }
            None => {
                self.gas_left = InternalGas::new(0);
                Err(PartialVMError::new(StatusCode::OUT_OF_GAS))
            }
        }
    }
}

fn get_simple_instruction_stack_change(
    instr: SimpleInstruction,
) -> (u64, u64, AbstractMemorySize, AbstractMemorySize) {
    use SimpleInstruction::*;

    match instr {
        // NB: The `Ret` pops are accounted for in `Call` instructions, so we say `Ret` has no pops.
        Nop | Ret => (0, 0, 0.into(), 0.into()),
        BrTrue | BrFalse => (1, 0, Type::Bool.size(), 0.into()),
        Branch => (0, 0, 0.into(), 0.into()),
        LdU8 => (0, 1, 0.into(), Type::U8.size()),
        LdU16 => (0, 1, 0.into(), Type::U16.size()),
        LdU32 => (0, 1, 0.into(), Type::U32.size()),
        LdU64 => (0, 1, 0.into(), Type::U64.size()),
        LdU128 => (0, 1, 0.into(), Type::U128.size()),
        LdU256 => (0, 1, 0.into(), Type::U256.size()),
        LdTrue | LdFalse => (0, 1, 0.into(), Type::Bool.size()),
        FreezeRef => (1, 1, REFERENCE_SIZE, REFERENCE_SIZE),
        ImmBorrowLoc | MutBorrowLoc => (0, 1, 0.into(), REFERENCE_SIZE),
        ImmBorrowField | MutBorrowField | ImmBorrowFieldGeneric | MutBorrowFieldGeneric => {
            (1, 1, REFERENCE_SIZE, REFERENCE_SIZE)
        }
        // Since we don't have the size of the value being cast here we take a conservative
        // over-approximation: it is _always_ getting cast from the smallest integer type.
        CastU8 => (1, 1, Type::U8.size(), Type::U8.size()),
        CastU16 => (1, 1, Type::U8.size(), Type::U16.size()),
        CastU32 => (1, 1, Type::U8.size(), Type::U32.size()),
        CastU64 => (1, 1, Type::U8.size(), Type::U64.size()),
        CastU128 => (1, 1, Type::U8.size(), Type::U128.size()),
        CastU256 => (1, 1, Type::U8.size(), Type::U256.size()),
        // NB: We don't know the size of what integers we're dealing with, so we conservatively
        // over-approximate by popping the smallest integers, and push the largest.
        Add | Sub | Mul | Mod | Div => (2, 1, Type::U8.size() + Type::U8.size(), Type::U256.size()),
        BitOr | BitAnd | Xor => (2, 1, Type::U8.size() + Type::U8.size(), Type::U256.size()),
        Shl | Shr => (2, 1, Type::U8.size() + Type::U8.size(), Type::U256.size()),
        Or | And => (
            2,
            1,
            Type::Bool.size() + Type::Bool.size(),
            Type::Bool.size(),
        ),
        Lt | Gt | Le | Ge => (2, 1, Type::U8.size() + Type::U8.size(), Type::Bool.size()),
        Not => (1, 1, Type::Bool.size(), Type::Bool.size()),
        Abort => (1, 0, Type::U64.size(), 0.into()),
    }
}

impl GasMeter for MoveOSGasMeter {
    fn balance_internal(&self) -> InternalGas {
        InternalGas::new(1000000)
    }

    fn charge_simple_instr(&mut self, instr: SimpleInstruction) -> PartialVMResult<()> {
        let (pops, pushes, pop_size, push_size) = get_simple_instruction_stack_change(instr);
        self.charge(1, pushes, pops, push_size.into(), pop_size.into())
    }

    fn charge_pop(&mut self, popped_val: impl ValueView) -> PartialVMResult<()> {
        self.charge(1, 0, 1, 0, popped_val.legacy_abstract_memory_size().into())
    }

    fn charge_call(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        args: impl ExactSizeIterator<Item = impl ValueView>,
        _num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        // We will have to perform this many pops for the call.
        let pops = args.len() as u64;
        // Size stays the same -- we're just moving it from the operand stack to the locals. But
        // the size on the operand stack is reduced by sum_{args} arg.size().
        let stack_reduction_size = args.fold(AbstractMemorySize::new(0), |acc, elem| {
            acc + elem.legacy_abstract_memory_size()
        });
        self.charge(1, 0, pops, 0, stack_reduction_size.into())
    }

    fn charge_call_generic(
        &mut self,
        _module_id: &ModuleId,
        _func_name: &str,
        _ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        args: impl ExactSizeIterator<Item = impl ValueView>,
        _num_locals: NumArgs,
    ) -> PartialVMResult<()> {
        // We have to perform this many pops from the operand stack for this function call.
        let pops = args.len() as u64;
        // Calculate the size reduction on the operand stack.
        let stack_reduction_size = args.fold(AbstractMemorySize::new(0), |acc, elem| {
            acc + elem.legacy_abstract_memory_size()
        });
        // Charge for the pops, no pushes, and account for the stack size decrease. Also track the
        // `CallGeneric` instruction we must have encountered for this.
        self.charge(1, 0, pops, 0, stack_reduction_size.into())
    }

    fn charge_ld_const(&mut self, size: NumBytes) -> PartialVMResult<()> {
        // Charge for the load from the locals onto the stack.
        self.charge(1, 1, 0, u64::from(size), 0)
    }

    fn charge_ld_const_after_deserialization(
        &mut self,
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        // We already charged for this based on the bytes that we're loading so don't charge again.
        Ok(())
    }

    fn charge_copy_loc(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        // Charge for the copy of the local onto the stack.
        self.charge(1, 1, 0, val.legacy_abstract_memory_size().into(), 0)
    }

    fn charge_move_loc(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        // Charge for the move of the local on to the stack. Note that we charge here since we
        // aren't tracking the local size (at least not yet). If we were, this should be a net-zero
        // operation in terms of memory usage.
        self.charge(1, 1, 0, val.legacy_abstract_memory_size().into(), 0)
    }

    fn charge_store_loc(&mut self, val: impl ValueView) -> PartialVMResult<()> {
        // Charge for the storing of the value on the stack into a local. Note here that if we were
        // also accounting for the size of the locals that this would be a net-zero operation in
        // terms of memory.
        self.charge(1, 0, 1, 0, val.legacy_abstract_memory_size().into())
    }

    fn charge_pack(
        &mut self,
        _is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        // We perform `num_fields` number of pops.
        let num_fields = args.len() as u64;
        // The actual amount of memory on the stack is staying the same with the addition of some
        // extra size for the struct, so the size doesn't really change much.
        self.charge(1, 1, num_fields, STRUCT_SIZE.into(), 0)
    }

    fn charge_unpack(
        &mut self,
        _is_generic: bool,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        // We perform `num_fields` number of pushes.
        let num_fields = args.len() as u64;
        self.charge(1, num_fields, 1, 0, STRUCT_SIZE.into())
    }

    fn charge_read_ref(&mut self, ref_val: impl ValueView) -> PartialVMResult<()> {
        // We read the the reference so we are decreasing the size of the stack by the size of the
        // reference, and adding to it the size of the value that has been read from that
        // reference.
        self.charge(
            1,
            1,
            1,
            ref_val.legacy_abstract_memory_size().into(),
            REFERENCE_SIZE.into(),
        )
    }

    fn charge_write_ref(
        &mut self,
        new_val: impl ValueView,
        old_val: impl ValueView,
    ) -> PartialVMResult<()> {
        // TODO(tzakian): We should account for this elsewhere as the owner of data the the
        // reference points to won't be on the stack. For now though, we treat it as adding to the
        // stack size.
        self.charge(
            1,
            1,
            2,
            new_val.legacy_abstract_memory_size().into(),
            old_val.legacy_abstract_memory_size().into(),
        )
    }

    fn charge_eq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        let size_reduction = lhs.legacy_abstract_memory_size() + rhs.legacy_abstract_memory_size();
        self.charge(
            1,
            1,
            2,
            (Type::Bool.size() + size_reduction).into(),
            size_reduction.into(),
        )
    }

    fn charge_neq(&mut self, lhs: impl ValueView, rhs: impl ValueView) -> PartialVMResult<()> {
        let size_reduction = lhs.legacy_abstract_memory_size() + rhs.legacy_abstract_memory_size();
        self.charge(1, 1, 2, Type::Bool.size().into(), size_reduction.into())
    }

    fn charge_borrow_global(
        &mut self,
        _is_mut: bool,
        _is_generic: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        self.charge(1, 1, 1, REFERENCE_SIZE.into(), Type::Address.size().into())
    }

    fn charge_exists(
        &mut self,
        _is_generic: bool,
        _ty: impl TypeView,
        // TODO(Gas): see if we can get rid of this param
        _exists: bool,
    ) -> PartialVMResult<()> {
        self.charge(
            1,
            1,
            1,
            Type::Bool.size().into(),
            Type::Address.size().into(),
        )
    }

    fn charge_move_from(
        &mut self,
        _is_generic: bool,
        _ty: impl TypeView,
        val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        let size = val
            .map(|val| val.legacy_abstract_memory_size())
            .unwrap_or_else(AbstractMemorySize::zero);
        self.charge(1, 1, 1, size.into(), Type::Address.size().into())
    }

    fn charge_move_to(
        &mut self,
        _is_generic: bool,
        _ty: impl TypeView,
        _val: impl ValueView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        self.charge(1, 0, 2, 0, Type::Address.size().into())
    }

    fn charge_vec_pack<'a>(
        &mut self,
        _ty: impl TypeView + 'a,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        // We will perform `num_args` number of pops.
        let num_args = args.len() as u64;
        // The amount of data on the stack stays constant except we have some extra metadata for
        // the vector to hold the length of the vector.
        self.charge(1, 1, num_args, VEC_SIZE.into(), 0)
    }

    fn charge_vec_len(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        self.charge(1, 1, 1, Type::U64.size().into(), REFERENCE_SIZE.into())
    }

    fn charge_vec_borrow(
        &mut self,
        _is_mut: bool,
        _ty: impl TypeView,
        _is_success: bool,
    ) -> PartialVMResult<()> {
        self.charge(
            1,
            1,
            2,
            REFERENCE_SIZE.into(),
            (REFERENCE_SIZE + Type::U64.size()).into(),
        )
    }

    fn charge_vec_push_back(
        &mut self,
        _ty: impl TypeView,
        _val: impl ValueView,
    ) -> PartialVMResult<()> {
        // The value was already on the stack, so we aren't increasing the number of bytes on the stack.
        self.charge(1, 0, 2, 0, REFERENCE_SIZE.into())
    }

    fn charge_vec_pop_back(
        &mut self,
        _ty: impl TypeView,
        _val: Option<impl ValueView>,
    ) -> PartialVMResult<()> {
        self.charge(1, 1, 1, 0, REFERENCE_SIZE.into())
    }

    fn charge_vec_unpack(
        &mut self,
        _ty: impl TypeView,
        expect_num_elements: NumArgs,
        _elems: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        // Charge for the pushes
        let pushes = u64::from(expect_num_elements);
        // The stack size stays pretty much the same modulo the additional vector size
        self.charge(1, pushes, 1, 0, VEC_SIZE.into())
    }

    fn charge_vec_swap(&mut self, _ty: impl TypeView) -> PartialVMResult<()> {
        let size_decrease = REFERENCE_SIZE + Type::U64.size() + Type::U64.size();
        self.charge(1, 1, 1, 0, size_decrease.into())
    }

    fn charge_load_resource(
        &mut self,
        _loaded: Option<(NumBytes, impl ValueView)>,
    ) -> PartialVMResult<()> {
        // We don't have resource loading so don't need to account for it.
        Ok(())
    }

    fn charge_native_function(
        &mut self,
        amount: InternalGas,
        ret_vals: Option<impl ExactSizeIterator<Item = impl ValueView>>,
    ) -> PartialVMResult<()> {
        // Charge for the number of pushes on to the stack that the return of this function is
        // going to cause.
        let pushes = ret_vals
            .as_ref()
            .map(|ret_vals| ret_vals.len())
            .unwrap_or(0) as u64;
        // Calculate the number of bytes that are getting pushed onto the stack.
        let size_increase = ret_vals
            .map(|ret_vals| {
                ret_vals.fold(AbstractMemorySize::zero(), |acc, elem| {
                    acc + elem.legacy_abstract_memory_size()
                })
            })
            .unwrap_or_else(AbstractMemorySize::zero);
        // Charge for the stack operations. We don't count this as an "instruction" since we
        // already accounted for the `Call` instruction in the
        // `charge_native_function_before_execution` call.
        self.charge(0, pushes, 0, size_increase.into(), 0)?;
        // Now charge the gas that the native function told us to charge.
        self.deduct_gas(amount)
    }

    fn charge_native_function_before_execution(
        &mut self,
        _ty_args: impl ExactSizeIterator<Item = impl TypeView>,
        args: impl ExactSizeIterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        // Determine the number of pops that are going to be needed for this function call, and
        // charge for them.
        let pops = args.len() as u64;
        // Calculate the size decrease of the stack from the above pops.
        let stack_reduction_size = args.fold(AbstractMemorySize::new(pops), |acc, elem| {
            acc + elem.legacy_abstract_memory_size()
        });
        // Track that this is going to be popping from the operand stack. We also increment the
        // instruction count as we need to account for the `Call` bytecode that initiated this
        // native call.
        self.charge(1, 0, pops, 0, stack_reduction_size.into())
    }

    fn charge_drop_frame(
        &mut self,
        _locals: impl Iterator<Item = impl ValueView>,
    ) -> PartialVMResult<()> {
        Ok(())
    }
}