use super::*;
use ckb_debugger_tests::{combine_lock_mol::*, blockchain::{BytesVec, WitnessArgs}};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{bytes::Bytes, core::Capacity, core::{TransactionBuilder, ScriptHashType}, packed::{Script, CellOutput, CellInput, CellDep}, prelude::*},
    context::Context,
};

const MAX_CYCLES: u64 = 10_000_000;

#[test]
fn test_pay_fee() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("pay-fee");
    let pf_out_point = context.deploy_cell(contract_bin);
    let as_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // prepare scripts
    let type_script = context.build_script(&as_out_point, Bytes::new()).unwrap();

    let type_script_hash = type_script.calc_script_hash();
    let max_fee: u64 = 10000000; // 0.1 CKB

    let lock_script = context
        .build_script(
            &pf_out_point,
            [
                type_script_hash.as_slice(),
                max_fee.to_le_bytes().as_slice(),
            ]
            .concat()
            .into(),
        )
        .unwrap();

    // prepare cells
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(Capacity::bytes(1000).unwrap().pack())
            .lock(lock_script.clone())
            .type_(Some(type_script.clone()).pack())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let outputs = vec![CellOutput::new_builder()
        .capacity((Capacity::bytes(1000).unwrap().as_u64() - 999).pack())
        .lock(lock_script.clone())
        .type_(Some(type_script.clone()).pack())
        .build()];

    // build transaction
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .output_data(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_auth() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("auth");
    let a_out_point = context.deploy_cell(contract_bin);

    // prepare scripts
    let lock_script = context
        .build_script(&a_out_point, vec![42, 24].into())
        .unwrap();

    // prepare cells
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(Capacity::bytes(1000).unwrap().pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let outputs = vec![CellOutput::new_builder()
        .capacity((Capacity::bytes(1000).unwrap().as_u64() - 999).pack())
        .lock(lock_script.clone())
        .build()];

    // build transaction
    let tx = TransactionBuilder::default()
        .input(input)
        .outputs(outputs)
        .output_data(Bytes::new().pack())
        .witness(vec![24, 42].pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_combine_pay_fee_auth() {
    // deploy contract
    let mut context = Context::default();
    let contract_bin: Bytes = Loader::default().load_binary("auth");
    let a_out_point = context.deploy_cell(contract_bin);
    let contract_bin: Bytes = Loader::default().load_binary("pay-fee");
    let pf_out_point = context.deploy_cell(contract_bin);
    let contract_bin: Bytes = Loader::default().load_binary("ckb-combine-lock");
    let ccl_out_point = context.deploy_cell(contract_bin);
    let as_out_point = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // prepare scripts
    let type_script = context.build_script(&as_out_point, Bytes::new()).unwrap();

    let type_script_hash = type_script.calc_script_hash();
    let max_fee: u64 = 10000000; // 0.1 CKB

    let pay_fee_script = context
        .build_script(
            &pf_out_point,
            [
                type_script_hash.as_slice(),
                max_fee.to_le_bytes().as_slice(),
            ]
            .concat()
            .into(),
        )
        .unwrap();

    let auth_script = context
        .build_script(&a_out_point, vec![42, 24].into())
        .unwrap();

    let child_script_config =
        build_child_script_config(&[auth_script, pay_fee_script], &[&[0], &[1]]);
    let mut hash = [0; 32];
    let mut blake2b = blake2b_rs::Blake2bBuilder::new(32)
        .personal(b"ckb-default-hash")
        .build();
    blake2b.update(child_script_config.as_slice());
    blake2b.finalize(&mut hash);

    let lock_script = context
        .build_script_with_hash_type(&ccl_out_point, ScriptHashType::Data2, Bytes::from(hash.to_vec()))
        .unwrap();

    // prepare cells
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(Capacity::bytes(1000).unwrap().pack())
            .lock(lock_script.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder()
        .previous_output(input_out_point)
        .build();
    let outputs = vec![CellOutput::new_builder()
        .capacity((Capacity::bytes(1000).unwrap().as_u64() - 999).pack())
        .lock(lock_script.clone())
        .build()];

    // prepare witnesses
    let child_script_config_opt = ChildScriptConfigOpt::new_builder()
        .set(Some(child_script_config))
        .build();
    let inner_witness = BytesVec::new_builder()
        .push(vec![24, 42].pack())
        .build();
    let combine_lock_witness = CombineLockWitness::new_builder()
        .index(Uint16::new_unchecked(0u16.to_le_bytes().to_vec().into()))
        .inner_witness(inner_witness)
        .script_config(child_script_config_opt)
        .build();

    let witness_args = WitnessArgs::new_builder()
        .lock(Some(combine_lock_witness.as_bytes()).pack())
        .build();

    // build transaction
    let tx = TransactionBuilder::default()
        .cell_dep(CellDep::new_builder().out_point(a_out_point).build())
        .input(input)
        .outputs(outputs)
        .output_data(Bytes::new().pack())
        .witness(witness_args.as_bytes().pack())
        .build();
    let tx = context.complete_tx(tx);

    // run
    let cycles = context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

fn build_child_script_config(scripts: &[Script], indexes: &[&[u8]]) -> ChildScriptConfig {
    let mut child_script_array_builder = ChildScriptArray::new_builder();
    for script in scripts {
        let child_script = ChildScript::new_builder()
            .code_hash(script.code_hash())
            .hash_type(script.hash_type())
            .args(script.args())
            .build();

        child_script_array_builder = child_script_array_builder.push(child_script);
    }

    let mut child_script_vec_vec_builder = ChildScriptVecVec::new_builder();
    for &index in indexes {
        let mut child_script_vec_builder = ChildScriptVec::new_builder();
        for i in index {
            child_script_vec_builder = child_script_vec_builder.push((*i).into());
        }
        child_script_vec_vec_builder = child_script_vec_vec_builder.push(child_script_vec_builder.build());
    }

    ChildScriptConfig::new_builder()
        .array(child_script_array_builder.build())
        .index(child_script_vec_vec_builder.build())
        .build()
}
