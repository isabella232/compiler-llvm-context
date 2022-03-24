//!
//! The LLVM generator function EVM data.
//!

use std::collections::BTreeMap;

use crate::context::function::block::Block;

///
/// The LLVM generator function EVM data.
///
#[derive(Debug, Clone)]
pub struct EVMData<'ctx> {
    /// The ordinary blocks with numeric tags.
    /// Is only used by the Solidity EVM compiler.
    pub blocks: BTreeMap<usize, Vec<Block<'ctx>>>,

    /// The function input size.
    pub input_size: usize,
    /// The function output size.
    pub output_size: usize,
    /// The function stack size.
    pub stack_size: usize,
}

impl<'ctx> EVMData<'ctx> {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(input_size: usize, output_size: usize, stack_size: usize) -> Self {
        Self {
            blocks: BTreeMap::new(),

            input_size,
            output_size,
            stack_size,
        }
    }

    ///
    /// Inserts a function block.
    ///
    pub fn insert_block(&mut self, tag: usize, block: Block<'ctx>) {
        if let Some(blocks) = self.blocks.get_mut(&tag) {
            blocks.push(block);
        } else {
            self.blocks.insert(tag, vec![block]);
        }
    }

    ///
    /// Returns the first block with the specified tag.
    ///
    /// Used to get the block for jumps and fallthroughs, where the block cloning is not required,
    /// unlike in cases with function call return addresses.
    ///
    pub fn first_block(
        &self,
        tag: usize,
    ) -> anyhow::Result<inkwell::basic_block::BasicBlock<'ctx>> {
        Ok(self
            .blocks
            .get(&tag)
            .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))?
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))?
            .inner)
    }

    ///
    /// Returns the block with the specified tag and call trace.
    ///
    /// Used to choose the correct function call return address block.
    ///
    /// If there is only one block, it is returned unconditionally.
    ///
    pub fn block_by_call_trace(
        &self,
        tag: usize,
        call_trace: &[usize],
    ) -> anyhow::Result<Block<'ctx>> {
        if self
            .blocks
            .get(&tag)
            .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))?
            .len()
            == 1
        {
            return self
                .blocks
                .get(&tag)
                .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))?
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag));
        }

        self.blocks
            .get(&tag)
            .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))?
            .iter()
            .find(|block| block.evm().call_trace.as_slice() == call_trace)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))
    }

    ///
    /// Returns the block with the specified tag and vertical tags buffer.
    ///
    /// Used to choose the correct function call return address block.
    ///
    /// If there is only one block, it is returned unconditionally.
    ///
    pub fn block_by_vertical_tags_buffer(
        &self,
        tag: usize,
        vertical_tags_buffer: &[usize],
    ) -> anyhow::Result<Block<'ctx>> {
        if self
            .blocks
            .get(&tag)
            .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))?
            .len()
            == 1
        {
            return self
                .blocks
                .get(&tag)
                .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))?
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag));
        }

        self.blocks
            .get(&tag)
            .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))?
            .iter()
            .find(|block| block.evm().vertical_tags_buffer.as_slice() == vertical_tags_buffer)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))
    }

    ///
    /// Returns all blocks with the specified tag.
    ///
    /// Used to map Ethereal IR blocks onto LLVM ones.
    ///
    pub fn all_blocks(
        &self,
        tag: usize,
    ) -> anyhow::Result<Vec<inkwell::basic_block::BasicBlock<'ctx>>> {
        Ok(self
            .blocks
            .get(&tag)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Undeclared function block {}", tag))?
            .into_iter()
            .map(|block| block.inner)
            .collect())
    }
}
