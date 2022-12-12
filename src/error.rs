use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StudentIntroError {
    #[error("Pda account passed in not initialized")]
    UninitializedAccount,
    #[error("Length of data passed exceeds max length")]
    InvalidDataLength,
    #[error("Pda account passed does not match the derived pda")]
    InvalidPda,
    #[error("Passed student name & stored student name don't match")]
    InvalidStudentName,
    #[error("At least one of the account passed is incorrect")]
    IncorrectAccountPassed,
}

impl From<StudentIntroError> for ProgramError {
    fn from(e: StudentIntroError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
