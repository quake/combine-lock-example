// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

use alloc::vec::Vec;

// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use ckb_std::{
    ckb_constants::Source,
    debug,
    high_level::{decode_hex, load_script, load_witness},
};

use crate::error::Error;

pub fn main() -> Result<(), Error> {
    let (script_args, witness) = load_script_args_and_witness()?;
    validate(script_args, witness)
}

fn validate(script_args: Vec<u8>, mut witness: Vec<u8>) -> Result<(), Error> {
    debug!("script_args is {:?}", script_args);
    debug!("witness is {:?}", witness);
    witness.reverse();
    if script_args == witness {
        Ok(())
    } else {
        Err(Error::WrongWitness)
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
