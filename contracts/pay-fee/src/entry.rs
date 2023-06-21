// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{
        decode_hex, load_cell_capacity, load_cell_lock_hash, load_cell_type_hash, load_script,
        load_witness, QueryIter,
    },
};

use crate::error::Error;

pub fn main() -> Result<(), Error> {
    let (script_args, _witness) = load_script_args_and_witness()?;
    validate(script_args)
}

fn validate(script_args: Vec<u8>) -> Result<(), Error> {
    debug!("script args is {:?}", script_args);

    if script_args.len() != 40 {
        return Err(Error::InvalidArgsLen);
    }

    let lock_script = load_script()?;
    let lock_script_hash: [u8; 32] = lock_script.calc_script_hash().unpack();
    let type_script_hash: [u8; 32] = script_args[0..32].try_into().unwrap();
    let max_fee: u64 = u64::from_le_bytes(script_args[32..40].try_into().unwrap());

    let iter = QueryIter::new(load_cell_lock_hash, Source::Input)
        .zip(QueryIter::new(load_cell_type_hash, Source::Input));
    let inputs: Vec<_> = iter
        .enumerate()
        .filter_map(|(index, (l_hash, t_hash))| {
            if l_hash == lock_script_hash && t_hash == Some(type_script_hash) {
                Some(index)
            } else {
                None
            }
        })
        .collect();
    if inputs.len() != 1 {
        return Err(Error::InvalidCellCount);
    }

    let iter = QueryIter::new(load_cell_lock_hash, Source::Output)
        .zip(QueryIter::new(load_cell_type_hash, Source::Output));
    let outputs: Vec<_> = iter
        .enumerate()
        .filter_map(|(index, (l_hash, t_hash))| {
            if l_hash == lock_script_hash && t_hash == Some(type_script_hash) {
                Some(index)
            } else {
                None
            }
        })
        .collect();
    if outputs.len() != 1 {
        return Err(Error::InvalidCellCount);
    }

    let input_capacity: u64 = load_cell_capacity(inputs[0], Source::Input)?;
    let output_capacity: u64 = load_cell_capacity(outputs[0], Source::Output)?;

    if input_capacity > output_capacity + max_fee {
        return Err(Error::InvalidFee);
    }
    Ok(())
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
