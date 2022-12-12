use borsh::BorshDeserialize;
use solana_program::program_error::ProgramError;

#[derive(BorshDeserialize)]
struct StudentIntroInstructionPayload {
    name: String,
    msg: String,
}
#[derive(BorshDeserialize)]
pub struct ReplyPayload {
    reply: String,
}

pub enum StudentIntroInstruction {
    AddStudentIntro { name: String, msg: String },
    UpdateStudentIntro { name: String, msg: String },
    AddReply { reply: String },
    InitializeMint,
}

impl StudentIntroInstruction {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = data
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match variant {
            0 => {
                let payload = StudentIntroInstructionPayload::try_from_slice(rest).unwrap();
                Self::AddStudentIntro {
                    name: payload.name,
                    msg: payload.msg,
                }
            }
            1 => {
                let payload = StudentIntroInstructionPayload::try_from_slice(rest).unwrap();
                Self::UpdateStudentIntro {
                    name: payload.name,
                    msg: payload.msg,
                }
            }
            2 => {
                let payload = ReplyPayload::try_from_slice(rest).unwrap();
                Self::AddReply {
                    reply: payload.reply,
                }
            }
            3 => Self::InitializeMint,
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}
