use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    instruction::Instruction,
    message::{v0, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::VersionedTransaction,
};
use spl_associated_token_account::{
    get_associated_token_address,
    instruction::{create_associated_token_account, create_associated_token_account_idempotent},
};
use spl_token::instruction::{close_account, sync_native};

use crate::common::accounts::PUBKEY_WSOL;

#[derive(Debug, Clone, Copy)]
pub struct PriorityFee {
    pub unit_limit: u32,
    pub unit_price: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct TipFee {
    pub tip_account: Pubkey,
    pub tip_lamports: u64,
}

pub fn build_transaction(
    payer: &Keypair,
    instructions: Vec<Instruction>,
    blockhash: Hash,
    fee: Option<PriorityFee>,
    tip: Option<TipFee>,
) -> anyhow::Result<VersionedTransaction> {
    let mut insts = vec![];
    if let Some(fee) = fee {
        insts.push(ComputeBudgetInstruction::set_compute_unit_price(fee.unit_price));
        insts.push(ComputeBudgetInstruction::set_compute_unit_limit(fee.unit_limit));
    }

    if let Some(tip) = tip {
        insts.push(system_instruction::transfer(&payer.pubkey(), &tip.tip_account, tip.tip_lamports));
    }

    insts.extend(instructions);

    let v0_message: v0::Message = v0::Message::try_compile(&payer.pubkey(), &insts, &[], blockhash)?;
    let versioned_message: VersionedMessage = VersionedMessage::V0(v0_message);
    let transaction = VersionedTransaction::try_new(versioned_message, &[&payer])?;

    Ok(transaction)
}

pub fn build_buy_instructions(payer: &Keypair, mint: &Pubkey, buy_instruction: Instruction) -> anyhow::Result<Vec<Instruction>> {
    let mut instructions = vec![];

    instructions.push(create_associated_token_account(&payer.pubkey(), &payer.pubkey(), &mint, &spl_token::ID));
    instructions.push(buy_instruction);

    Ok(instructions)
}

pub fn build_sell_instructions(payer: &Keypair, mint: &Pubkey, sell_instruction: Instruction, close_mint_ata: bool) -> Result<Vec<Instruction>, anyhow::Error> {
    let mut instructions = vec![sell_instruction];

    if close_mint_ata {
        let mint_ata = get_associated_token_address(&payer.pubkey(), &mint);
        instructions.push(close_account(&spl_token::ID, &mint_ata, &payer.pubkey(), &payer.pubkey(), &[&payer.pubkey()])?);
    }

    Ok(instructions)
}

pub fn build_wsol_buy_instructions(payer: &Keypair, mint: &Pubkey, amount_sol: u64, buy_instruction: Instruction) -> anyhow::Result<Vec<Instruction>> {
    let mut instructions = vec![];

    instructions.push(create_associated_token_account_idempotent(
        &payer.pubkey(),
        &payer.pubkey(),
        &mint,
        &spl_token::ID,
    ));

    instructions.push(create_associated_token_account_idempotent(
        &payer.pubkey(),
        &payer.pubkey(),
        &PUBKEY_WSOL,
        &spl_token::ID,
    ));

    let wsol_ata = get_associated_token_address(&payer.pubkey(), &PUBKEY_WSOL);
    instructions.push(system_instruction::transfer(&payer.pubkey(), &wsol_ata, amount_sol));

    instructions.push(sync_native(&spl_token::ID, &wsol_ata).unwrap());

    instructions.push(buy_instruction);

    instructions.push(close_account(&spl_token::ID, &wsol_ata, &payer.pubkey(), &payer.pubkey(), &[&payer.pubkey()]).unwrap());

    Ok(instructions)
}

pub fn build_wsol_sell_instructions(payer: &Keypair, mint: &Pubkey, close_mint_ata: bool, sell_instruction: Instruction) -> anyhow::Result<Vec<Instruction>> {
    let mint_ata = get_associated_token_address(&payer.pubkey(), &mint);
    let wsol_ata = get_associated_token_address(&payer.pubkey(), &PUBKEY_WSOL);

    let mut instructions = vec![];
    instructions.push(create_associated_token_account_idempotent(
        &payer.pubkey(),
        &payer.pubkey(),
        &PUBKEY_WSOL,
        &spl_token::ID,
    ));

    instructions.push(sell_instruction);

    instructions.push(close_account(&spl_token::ID, &wsol_ata, &payer.pubkey(), &payer.pubkey(), &[&payer.pubkey()]).unwrap());

    if close_mint_ata {
        instructions.push(close_account(&spl_token::ID, &mint_ata, &payer.pubkey(), &payer.pubkey(), &[&payer.pubkey()]).unwrap());
    }

    Ok(instructions)
}
