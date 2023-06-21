// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::{
    ckb_constants::Source,
    ckb_types::prelude::*,
    debug,
    high_level::{
        decode_hex, load_cell, load_cell_data, load_cell_lock_hash, load_script, load_witness,
        QueryIter,
    },
};

use crate::error::Error;

pub fn main() -> Result<(), Error> {
    let (script_args, witness) = load_script_args_and_witness()?;
    validate(script_args, witness)
}

fn validate(script_args: Vec<u8>, witness: Vec<u8>) -> Result<(), Error> {
    debug!("script_args is {:?}", script_args);
    debug!("witness is {:?}", witness);

    let lock_script = load_script()?;
    let lock_script_hash: [u8; 32] = lock_script.calc_script_hash().unpack();

    let iter = QueryIter::new(load_cell_lock_hash, Source::Input);
    let inputs: Vec<_> = iter
        .enumerate()
        .filter_map(|(index, l_hash)| {
            if l_hash == lock_script_hash {
                Some(index)
            } else {
                None
            }
        })
        .collect();
    if inputs.len() != 1 {
        return Err(Error::InvalidCellCount);
    }

    let iter = QueryIter::new(load_cell_lock_hash, Source::Output);
    let outputs: Vec<_> = iter
        .enumerate()
        .filter_map(|(index, l_hash)| {
            if l_hash == lock_script_hash {
                Some(index)
            } else {
                None
            }
        })
        .collect();
    if outputs.len() != 1 {
        return Err(Error::InvalidCellCount);
    }

    let input = load_cell(inputs[0], Source::Input)?;
    let output = load_cell(outputs[0], Source::Output)?;
    if input.as_slice() != output.as_slice() {
        return Err(Error::CellChanged);
    }

    let input_data = load_cell_data(inputs[0], Source::Input)?;
    let output_data = load_cell_data(outputs[0], Source::Output)?;
    validate_state_transition(&input_data, &output_data)
}

fn validate_state_transition(input_data: &[u8], output_data: &[u8]) -> Result<(), Error> {
    match (input_data, output_data) {
        ([], [1]) => Ok(()),
        ([1], [2]) => Ok(()),
        ([1], [3]) => Ok(()),
        ([2], [4]) => Ok(()),
        _ => Err(Error::InvalidStateTransition),
    }
}

fn load_script_args_and_witness() -> Result<(Vec<u8>, Vec<u8>), Error> {
    debug!("argv len {}", ckb_std::env::argv().len());
    if ckb_std::env::argv().len() == 0 {
        Ok((
            load_script()?.args().raw_data().to_vec(),
            load_witness(0, Source::GroupInput)?,
        ))
    } else if ckb_std::env::argv().len() == 2 {
        let script_args = decode_hex(&ckb_std::env::argv()[0])?;
        let witness = decode_hex(&ckb_std::env::argv()[1])?;
        Ok((script_args, witness))
    } else {
        Err(Error::WrongArgv)
    }
}
