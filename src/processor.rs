use crate::error::StudentIntroError;
use crate::instruction::StudentIntroInstruction;
use crate::state::{StudentIntroAccountState, StudentIntroReply, StudentIntroReplyCounter};
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    native_token::LAMPORTS_PER_SOL,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    system_instruction,
    system_program::ID as SYSTEM_PROGRAM_ID,
    sysvar::{rent::Rent, rent::ID as RENT_PROGRAM_ID, Sysvar},
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    instruction::{initialize_mint, mint_to},
    ID as TOKEN_PROGRAM_ID,
};
use std::convert::TryInto;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = StudentIntroInstruction::unpack(instruction_data)?;
    match instruction {
        StudentIntroInstruction::AddStudentIntro { name, msg } => {
            add_student_intro(program_id, accounts, name, msg)
        }
        StudentIntroInstruction::UpdateStudentIntro { name, msg } => {
            update_student_intro(program_id, accounts, name, msg)
        }
        StudentIntroInstruction::AddReply { reply } => add_reply(program_id, accounts, reply),
        StudentIntroInstruction::InitializeMint => initialize_mint_account(program_id, accounts),
    }
}

pub fn add_student_intro(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    msg: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;
    let counter_account = next_account_info(account_info_iter)?;
    let mint_account = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?; //mint authority, a program PDA
    let user_ata = next_account_info(account_info_iter)?; //initializer's associated token acc
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    if !initializer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, bump) = Pubkey::find_program_address(
        &[initializer.key.as_ref(), name.as_bytes().as_ref()],
        program_id,
    );
    let (mint_pda, _mint_bump) = Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, mint_auth_bump) =
        Pubkey::find_program_address(&[b"token_auth"], program_id);
    msg!("Found PDA: {}", pda);
    if pda != *pda_account.key {
        return Err(StudentIntroError::InvalidPda.into());
    }
    if mint_pda != *mint_account.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if mint_auth_pda != *mint_auth.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if *user_ata.key != get_associated_token_address(initializer.key, mint_account.key) {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if TOKEN_PROGRAM_ID != *token_program.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if SYSTEM_PROGRAM_ID != *system_program.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    let data_len = 1000;
    if StudentIntroAccountState::get_account_size(name.clone(), msg.clone()) > data_len {
        return Err(StudentIntroError::InvalidDataLength.into());
    }
    let rent_amt = Rent::get()?.minimum_balance(data_len);

    invoke_signed(
        &system_instruction::create_account(
            &initializer.key,
            &pda_account.key,
            rent_amt,
            data_len.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            pda_account.clone(),
            system_program.clone(),
        ],
        &[&[initializer.key.as_ref(), name.as_bytes().as_ref(), &[bump]]],
    )?;
    msg!("Created PDA account successfully");
    msg!("Deserializing account data");
    msg!("Name: {}", name.clone());
    msg!("Msg: {}", msg.clone());
    let mut account_data =
        try_from_slice_unchecked::<StudentIntroAccountState>(&pda_account.data.borrow()).unwrap();
    if account_data.is_initialized() {
        msg!("PDA account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    account_data.discriminator = StudentIntroAccountState::DISCRIMINATOR.to_string();
    account_data.identity = *initializer.key;
    account_data.name = name;
    account_data.msg = msg;
    account_data.is_initialized = true;
    msg!("Serializing account data");
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    msg!("Serialization successful");

    let (counter_pda, counter_bump) =
        Pubkey::find_program_address(&[pda.as_ref(), "reply".as_ref()], program_id);
    if counter_pda != *counter_account.key {
        msg!("invalid seeds for counter PDA");
        return Err(ProgramError::InvalidArgument);
    }
    let rent_amt = Rent::get()?.minimum_balance(StudentIntroReplyCounter::SIZE);
    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            counter_account.key,
            rent_amt,
            StudentIntroReplyCounter::SIZE.try_into().unwrap(),
            program_id,
        ),
        &[
            initializer.clone(),
            counter_account.clone(),
            system_program.clone(),
        ],
        &[&[pda.as_ref(), "reply".as_ref(), &[counter_bump]]],
    )?;
    msg!("created counter PDA");
    let mut counter_data =
        try_from_slice_unchecked::<StudentIntroReplyCounter>(&counter_account.data.borrow())
            .unwrap();
    if counter_data.is_initialized() {
        msg!("counter PDA already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    counter_data.discriminator = StudentIntroReplyCounter::DISCRIMINATOR.to_string();
    counter_data.is_initialized = true;
    counter_data.counter = 0;
    counter_data.serialize(&mut &mut counter_account.data.borrow_mut()[..])?;
    msg!("serialized counter PDA");

    //token mint logic
    msg!(
        "minting 10 tokens({:?}) to {:?}",
        mint_account.key,
        initializer.key
    );
    invoke_signed(
        &mint_to(
            token_program.key,
            mint_account.key,
            user_ata.key,
            mint_auth.key,
            &[],
            10 * LAMPORTS_PER_SOL, //our token has 9 decimals, same as SOL
        )?,
        &[mint_account.clone(), user_ata.clone(), mint_auth.clone()],
        &[&[b"token_auth", &[mint_auth_bump]]],
    )?;
    msg!("token mint successful");
    Ok(())
}

pub fn update_student_intro(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    msg: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;
    if !initializer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if pda_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    msg!("Deserializing account data");
    let mut account_data =
        try_from_slice_unchecked::<StudentIntroAccountState>(&pda_account.data.borrow()).unwrap();
    let (pda, _bump) = Pubkey::find_program_address(
        &[
            initializer.key.as_ref(),
            account_data.name.as_bytes().as_ref(),
        ],
        program_id,
    );
    if pda != *pda_account.key {
        return Err(StudentIntroError::InvalidPda.into());
    }
    if !account_data.is_initialized() {
        return Err(StudentIntroError::UninitializedAccount.into());
    }
    if account_data.name != name {
        return Err(StudentIntroError::InvalidStudentName.into());
    }
    if 1 + 4 + name.len() + 4 + msg.len() > 1000 {
        return Err(StudentIntroError::InvalidDataLength.into());
    }
    account_data.msg = msg;
    msg!("Serializing account data");
    account_data.serialize(&mut &mut pda_account.data.borrow_mut()[..])?;
    Ok(())
}

pub fn add_reply(program_id: &Pubkey, accounts: &[AccountInfo], reply: String) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let replier = next_account_info(account_info_iter)?;
    let intro_account = next_account_info(account_info_iter)?;
    let counter_account = next_account_info(account_info_iter)?;
    let reply_account = next_account_info(account_info_iter)?;
    let mint_account = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let user_ata = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    if !replier.is_signer {
        msg!("replier needs to sign the txn");
        return Err(ProgramError::MissingRequiredSignature);
    }
    if intro_account.owner != program_id {
        msg!("PDA account not owned by program");
        return Err(ProgramError::IllegalOwner);
    }
    let intro_data =
        try_from_slice_unchecked::<StudentIntroAccountState>(&intro_account.data.borrow()).unwrap();
    if !intro_data.is_initialized() {
        msg!("PDA account not initialized");
        return Err(StudentIntroError::UninitializedAccount.into());
    }
    let (pda, _) = Pubkey::find_program_address(
        &[intro_data.identity.as_ref(), intro_data.name.as_ref()],
        program_id,
    );
    if pda != *intro_account.key {
        msg!("Invalid PDA account passed");
        return Err(ProgramError::InvalidArgument);
    }
    if counter_account.owner != program_id {
        msg!("counter account not owned by program");
        return Err(ProgramError::IllegalOwner);
    }
    let (counter_pda, _) =
        Pubkey::find_program_address(&[pda.as_ref(), "reply".as_ref()], program_id);
    if counter_pda != *counter_account.key {
        msg!("Invalid counter account passed");
        return Err(ProgramError::InvalidArgument);
    }
    let mut counter_data =
        try_from_slice_unchecked::<StudentIntroReplyCounter>(&counter_account.data.borrow())
            .unwrap();
    let reply_count = counter_data.counter;
    let (reply_pda, reply_bump) = Pubkey::find_program_address(
        &[pda.as_ref(), reply_count.to_be_bytes().as_ref()],
        program_id,
    );
    if reply_pda != *reply_account.key {
        msg!("Invalid seeds for reply PDA");
        return Err(StudentIntroError::InvalidPda.into());
    }
    let (mint_pda, _mint_bump) = Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, mint_auth_bump) =
        Pubkey::find_program_address(&[b"token_auth"], program_id);
    if mint_pda != *mint_account.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if mint_auth_pda != *mint_auth.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if *user_ata.key != get_associated_token_address(replier.key, mint_account.key) {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if TOKEN_PROGRAM_ID != *token_program.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if SYSTEM_PROGRAM_ID != *system_program.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }

    msg!("found reply PDA: {}", reply_pda);
    let account_size = StudentIntroReply::get_account_size(reply.clone());
    let rent_amt = Rent::get()?.minimum_balance(account_size);
    invoke_signed(
        &system_instruction::create_account(
            replier.key,
            reply_account.key,
            rent_amt,
            account_size.try_into().unwrap(),
            program_id,
        ),
        &[
            replier.clone(),
            reply_account.clone(),
            system_program.clone(),
        ],
        &[&[
            pda.as_ref(),
            reply_count.to_be_bytes().as_ref(),
            &[reply_bump],
        ]],
    )?;
    msg!("created reply PDA account");
    let mut reply_data =
        try_from_slice_unchecked::<StudentIntroReply>(&reply_account.data.borrow()).unwrap();
    if reply_data.is_initialized() {
        msg!("reply account already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    reply_data.discriminator = StudentIntroReply::DISCRIMINATOR.to_string();
    reply_data.is_initialized = true;
    reply_data.intro = *intro_account.key;
    reply_data.replier = *replier.key;
    reply_data.reply = reply;
    reply_data.count = reply_count;
    reply_data.serialize(&mut &mut reply_account.data.borrow_mut()[..])?;
    msg!("serialized reply PDA");
    counter_data.counter += 1;
    counter_data.serialize(&mut &mut counter_account.data.borrow_mut()[..])?;
    msg!("serialized counter PDA");

    //token mint logic
    msg!(
        "minting 5 tokens{:?} to {:?}",
        mint_account.key,
        replier.key
    );
    invoke_signed(
        &mint_to(
            token_program.key,
            mint_account.key,
            user_ata.key,
            mint_auth.key,
            &[],
            5 * LAMPORTS_PER_SOL,
        )?,
        &[mint_account.clone(), user_ata.clone(), mint_auth.clone()],
        &[&[b"token_auth", &[mint_auth_bump]]],
    )?;
    msg!("successfully minted 5 tokens to replier");
    Ok(())
}

pub fn initialize_mint_account(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let initializer = next_account_info(account_info_iter)?;
    let mint_account = next_account_info(account_info_iter)?;
    let mint_auth = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let sysvar_rent = next_account_info(account_info_iter)?;

    if !initializer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let (mint_pda, mint_bump) = Pubkey::find_program_address(&[b"token_mint"], program_id);
    let (mint_auth_pda, _mint_auth_bump) =
        Pubkey::find_program_address(&[b"token_auth"], program_id);
    if mint_pda != *mint_account.key || mint_auth_pda != *mint_auth.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if SYSTEM_PROGRAM_ID != *system_program.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if TOKEN_PROGRAM_ID != *token_program.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    if RENT_PROGRAM_ID != *sysvar_rent.key {
        return Err(StudentIntroError::IncorrectAccountPassed.into());
    }
    //length of spl mint account is 82
    let rent_amt = Rent::get()?.minimum_balance(82);
    msg!("creating mint account: {:?}", mint_account.key);
    invoke_signed(
        &system_instruction::create_account(
            initializer.key,
            mint_account.key,
            rent_amt,
            82,
            token_program.key,
        ),
        &[
            initializer.clone(),
            mint_account.clone(),
            system_program.clone(),
        ],
        &[&[b"token_mint", &[mint_bump]]],
    )?;
    msg!("created mint account successfully");
    msg!("initializing mint account");
    invoke_signed(
        &initialize_mint(
            token_program.key,
            mint_account.key,
            mint_auth.key,
            Option::None,
            9,
        )?,
        &[mint_account.clone(), sysvar_rent.clone(), mint_auth.clone()],
        &[&[b"token_mint", &[mint_bump]]],
    )?;
    msg!("initialized mint account successfully");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        system_program::ID as SYSTEM_PROGRAM_ID,
    };
    use spl_associated_token_account::{
        get_associated_token_address, instruction::create_associated_token_account,
    };
    use spl_token::ID as TOKEN_PROGRAM_ID;
    use {
        assert_matches::*,
        solana_program_test::*,
        solana_sdk::{
            signature::Signer, sysvar::rent::ID as SYSVAR_RENT_ID, transaction::Transaction,
        },
    };

    fn create_initialize_mint_ix(
        initializer: Pubkey,
        program_id: Pubkey,
    ) -> (Pubkey, Pubkey, Instruction) {
        let (mint, _mint_bump) = Pubkey::find_program_address(&[b"token_mint"], &program_id);
        let (mint_auth, _mint_auth_bump) =
            Pubkey::find_program_address(&[b"token_auth"], &program_id);
        let ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(initializer, true),
                AccountMeta::new(mint, false),
                AccountMeta::new(mint_auth, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(SYSVAR_RENT_ID, false),
            ],
            data: vec![3],
        };
        (mint, mint_auth, ix)
    }
    fn create_add_intro_ix(
        payer: Pubkey,
        program_id: Pubkey,
        name: String,
        msg: String,
    ) -> Instruction {
        let (mint, _mint_bump) = Pubkey::find_program_address(&[b"token_mint"], &program_id);
        let (mint_auth, _mint_auth_bump) =
            Pubkey::find_program_address(&[b"token_auth"], &program_id);
        let (intro_pda, _intro_bump) =
            Pubkey::find_program_address(&[payer.as_ref(), name.as_ref()], &program_id);
        let (counter_pda, _counter_bump) =
            Pubkey::find_program_address(&[intro_pda.as_ref(), b"reply"], &program_id);
        let ata = get_associated_token_address(&payer, &mint);
        let mut data = vec![0];
        data.append(
            &mut (TryInto::<u32>::try_into(name.len()).unwrap().to_le_bytes())
                .try_into()
                .unwrap(),
        );
        data.append(&mut name.into_bytes());
        data.append(
            &mut (TryInto::<u32>::try_into(msg.len()).unwrap().to_le_bytes())
                .try_into()
                .unwrap(),
        );
        data.append(&mut msg.into_bytes());
        let accounts = vec![
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new(intro_pda, false),
            AccountMeta::new(counter_pda, false),
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(mint_auth, false),
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ];
        Instruction {
            program_id,
            accounts,
            data,
        }
    }

    #[tokio::test]
    async fn test_init_mint_acc_ix() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "student intro program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;

        let (_mint, _mint_auth, ix) = create_initialize_mint_ix(payer.pubkey(), program_id);
        let mut tx = Transaction::new_with_payer(&[ix], Some(&payer.pubkey()));
        tx.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(tx).await, Ok(_));
    }

    #[tokio::test]
    async fn test_add_student_intro_ix() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "student intro program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;
        let name = "Naruto".to_owned();
        let msg = "Developing solana jutsu".to_owned();
        let (mint, _mint_auth, init_mint_ix) =
            create_initialize_mint_ix(payer.pubkey(), program_id);
        let create_ata_ix = create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint,
            &TOKEN_PROGRAM_ID,
        );
        let add_intro_ix = create_add_intro_ix(payer.pubkey(), program_id, name, msg);
        let mut tx = Transaction::new_with_payer(
            &[init_mint_ix, create_ata_ix, add_intro_ix],
            Some(&payer.pubkey()),
        );
        tx.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(tx).await, Ok(_));
    }

    #[tokio::test]
    async fn test_update_student_intro_ix() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "student intro program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;
        let name = "Naruto".to_owned();
        let (prev_msg, new_msg) = (
            "Looking to develop solana jutsu".to_owned(),
            "Exploring solana ecosystem".to_owned(),
        );
        let (mint, _mint_auth, init_mint_ix) =
            create_initialize_mint_ix(payer.pubkey(), program_id);
        let create_ata_ix = create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint,
            &TOKEN_PROGRAM_ID,
        );
        let add_intro_ix = create_add_intro_ix(payer.pubkey(), program_id, name.clone(), prev_msg);
        let (intro_pda, _intro_bump) =
            Pubkey::find_program_address(&[payer.pubkey().as_ref(), name.as_ref()], &program_id);
        let mut data = vec![1];
        data.append(
            &mut (TryInto::<u32>::try_into(name.len()).unwrap().to_le_bytes())
                .try_into()
                .unwrap(),
        );
        data.append(&mut name.into_bytes());
        data.append(
            &mut (TryInto::<u32>::try_into(new_msg.len())
                .unwrap()
                .to_le_bytes())
            .try_into()
            .unwrap(),
        );
        data.append(&mut new_msg.into_bytes());
        let update_intro_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(payer.pubkey(), true),
                AccountMeta::new(intro_pda, false),
            ],
            data,
        };
        let mut tx = Transaction::new_with_payer(
            &[init_mint_ix, create_ata_ix, add_intro_ix, update_intro_ix],
            Some(&payer.pubkey()),
        );
        tx.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(tx).await, Ok(_));
    }

    #[tokio::test]
    async fn test_add_reply_ix() {
        let program_id = Pubkey::new_unique();
        let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
            "student intro program",
            program_id,
            processor!(process_instruction),
        )
        .start()
        .await;
        let name = "Naruto".to_owned();
        let msg = "Looking to develop solana jutsu".to_owned();
        let reply = "All the best Naruto".to_owned();

        let (mint, mint_auth, init_mint_ix) = create_initialize_mint_ix(payer.pubkey(), program_id);
        let create_ata_ix = create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint,
            &TOKEN_PROGRAM_ID,
        );
        let add_intro_ix = create_add_intro_ix(payer.pubkey(), program_id, name.clone(), msg);
        let (intro_pda, _intro_bump) =
            Pubkey::find_program_address(&[payer.pubkey().as_ref(), name.as_ref()], &program_id);
        let (counter_pda, _counter_bump) =
            Pubkey::find_program_address(&[intro_pda.as_ref(), b"reply"], &program_id);
        let (reply_pda, _reply_bump) = Pubkey::find_program_address(
            &[intro_pda.as_ref(), 0u64.to_be_bytes().as_ref()],
            &program_id,
        );
        let ata = get_associated_token_address(&payer.pubkey(), &mint);

        let mut data = vec![2];
        data.append(
            &mut (TryInto::<u32>::try_into(reply.len()).unwrap().to_le_bytes())
                .try_into()
                .unwrap(),
        );
        data.append(&mut reply.into_bytes());
        let add_reply_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(payer.pubkey(), true),
                AccountMeta::new_readonly(intro_pda, false),
                AccountMeta::new(counter_pda, false),
                AccountMeta::new(reply_pda, false),
                AccountMeta::new(mint, false),
                AccountMeta::new_readonly(mint_auth, false),
                AccountMeta::new(ata, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            ],
            data,
        };

        let mut tx = Transaction::new_with_payer(
            &[init_mint_ix, create_ata_ix, add_intro_ix, add_reply_ix],
            Some(&payer.pubkey()),
        );
        tx.sign(&[&payer], recent_blockhash);
        assert_matches!(banks_client.process_transaction(tx).await, Ok(_));
    }
}
