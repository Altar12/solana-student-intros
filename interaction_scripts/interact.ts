import { initializeKeypair } from "./initializeKeypair"
import * as web3 from "@solana/web3.js"
import * as borsh from "@project-serum/borsh"
import * as token from "@solana/spl-token"
import { StudentIntro } from "./models/StudentIntro"
import { StudentIntroCounter } from "./models/StudentIntroCounter"
import { StudentIntroReply } from "./models/StudentIntroReply"
import BN from "bn.js"

const programId = new web3.PublicKey("9nKhQhLdUq5z37SjiqmsNGGEXDUmLsoryek1gvWxUKsg")
const introInstructionLayout = borsh.struct([
  borsh.u8("variant"),
  borsh.str("name"),
  borsh.str("msg"),
])
const replyInstructionLayout = borsh.struct([
  borsh.u8("variant"),
  borsh.str("reply"),
])

async function addIntro(
  user: web3.Keypair,
  connection: web3.Connection,
  name: string,
  msg: string
) {
  let buffer = Buffer.alloc(1000);
  introInstructionLayout.encode( { variant: 0, name, msg, }, buffer)
  buffer = buffer.slice(0, introInstructionLayout.getSpan(buffer))
  let [intro_pda] = await web3.PublicKey.findProgramAddress([user.publicKey.toBuffer(), Buffer.from(name)], programId)
  console.log("Intro PDA is", intro_pda.toBase58())
  let [counter_pda] = await web3.PublicKey.findProgramAddress([intro_pda.toBuffer(), Buffer.from("reply")], programId)
  console.log("Counter PDA is", counter_pda.toBase58())
  let [mint_pda] = await web3.PublicKey.findProgramAddress([Buffer.from("token_mint")], programId)
  let [mint_auth_pda] = await web3.PublicKey.findProgramAddress([Buffer.from("token_auth")], programId)
  let userTokenAcc = await token.getAssociatedTokenAddress(mint_pda, user.publicKey)
  const instruction = new web3.TransactionInstruction({
    keys: [
      {
        pubkey: user.publicKey,
        isSigner: true,
        isWritable: true
      },
      {
        pubkey: intro_pda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: counter_pda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: mint_pda,
        isSigner: false,
        isWritable: true
      },
      {
        pubkey: mint_auth_pda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: userTokenAcc,
        isSigner: false,
        isWritable: true
      },
      {
        pubkey: web3.SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: token.TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false
      }
    ],
    programId,
    data: buffer
  })
  const transaction = new web3.Transaction()
  transaction.add(instruction)
  await web3.sendAndConfirmTransaction(connection, transaction, [user])
}


async function updateIntro(
  user: web3.Keypair,
  connection: web3.Connection,
  name: string,
  msg: string
) {
  let buffer = Buffer.alloc(1000);
  introInstructionLayout.encode( { variant: 1, name, msg, }, buffer)
  buffer = buffer.slice(0, introInstructionLayout.getSpan(buffer))
  let [intro_pda] = await web3.PublicKey.findProgramAddress([user.publicKey.toBuffer(), Buffer.from(name)], programId)
  console.log("Intro PDA is", intro_pda.toBase58())
  const instruction = new web3.TransactionInstruction({
    keys: [
      {
        pubkey: user.publicKey,
        isSigner: true,
        isWritable: true
      },
      {
        pubkey: intro_pda,
        isSigner: false,
        isWritable: true,
      }
    ],
    programId,
    data: buffer
  })
  const transaction = new web3.Transaction()
  transaction.add(instruction)
  await web3.sendAndConfirmTransaction(connection, transaction, [user])
}

async function addReply(
  user: web3.Keypair,
  connection: web3.Connection,
  name: string,
  reply: string
) {
  let buffer = Buffer.alloc(1000)
  replyInstructionLayout.encode({ variant: 2, reply}, buffer)
  buffer = buffer.slice(0, replyInstructionLayout.getSpan(buffer))
  let [intro_pda] = await web3.PublicKey.findProgramAddress([user.publicKey.toBuffer(), Buffer.from(name)], programId)
  console.log("Intro PDA:", intro_pda.toBase58())
  let [counter_pda] = await web3.PublicKey.findProgramAddress([intro_pda.toBuffer(), Buffer.from("reply")], programId)
  let [mint_pda] = await web3.PublicKey.findProgramAddress([Buffer.from("token_mint")], programId)
  let [mint_auth_pda] = await web3.PublicKey.findProgramAddress([Buffer.from("token_auth")], programId)
  let userTokenAcc = await token.getAssociatedTokenAddress(mint_pda, user.publicKey)
  console.log("Counter PDA:", counter_pda.toBase58())
  const account = await connection.getAccountInfo(counter_pda)
  console.log("counter account", account);
  const counter = StudentIntroCounter.deserialize(account?.data)
  console.log(counter)
  if (!counter) {
    console.log("no counter account found")
    return;
  }
  let [reply_pda] = await web3.PublicKey.findProgramAddress([intro_pda.toBuffer(), new BN(counter.counter).toArrayLike(Buffer, "be", 8)], programId)
  console.log("Reply PDA:", reply_pda.toBase58())
  const instruction = new web3.TransactionInstruction({
    keys: [
      {
        pubkey: user.publicKey,
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: intro_pda,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: counter_pda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: reply_pda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: mint_pda,
        isSigner: false,
        isWritable: true
      },
      {
        pubkey: mint_auth_pda,
        isSigner: false,
        isWritable: false
      },
      {
        pubkey: userTokenAcc,
        isSigner: false,
        isWritable: true
      },
      {
        pubkey: web3.SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: token.TOKEN_PROGRAM_ID,
        isSigner: false,
        isWritable: false
      }
    ],
    programId,
    data: buffer
  })
  const transaction = new web3.Transaction()
  transaction.add(instruction)
  await web3.sendAndConfirmTransaction(connection, transaction, [user])
}

async function getIntros(
  connection: web3.Connection
): Promise<StudentIntro[]> {
  const accounts = await connection.getProgramAccounts(programId)
  const introAccounts = accounts.filter((account) => {
    let buffer = account.account.data.slice(0, 9)
    console.log("Discriminator", buffer.toString())
    return buffer.toString().includes("intro")
  })
  const studentIntros = introAccounts.map((account) => {
    let buffer = account.account.data
    let intro = StudentIntro.deserialize(buffer)
    if (!intro) return new StudentIntro("invalid account", false, web3.Keypair.generate().publicKey, "dummy", "dummy")
    return intro
  })
  return studentIntros
}

async function getReplies(
  connection: web3.Connection,
  intro: web3.PublicKey
): Promise<StudentIntroReply[]> {
  const accounts = await connection.getProgramAccounts(programId)
  const replyAccounts = accounts.filter((account) => {
    let buffer = account.account.data.slice(0, 9)
    return buffer.toString().includes("reply")
  })
  const allReplies = replyAccounts.map((account) => {
    let buffer = account.account.data
    let reply = StudentIntroReply.deserialize(buffer)
    if (!reply) return new StudentIntroReply("invalid account", false, web3.SystemProgram.programId, web3.SystemProgram.programId, "", 0)
    return reply
  })
  const introReplies = allReplies.filter(reply => reply.intro.includes(intro.toBase58()))
  return introReplies
}

async function main() {
  const connection = new web3.Connection("http://127.0.0.1:8899")
  const user = await initializeKeypair(connection)
  console.log("PublicKey:", user.publicKey.toBase58())
  await addIntro(user, connection, "Neji", "Exploring solana ecosystem")
  await addIntro(user, connection, "Lee", "Diving deep with solana")
  await addIntro(user, connection, "Tenten", "Mastering blockchain with solana")
  await updateIntro(user, connection, "Lee", "Learning solana to setup my solana school")
  await addReply(user, connection, "Lee", "All the best")
  await addReply(user, connection, "Lee", "Good luck")
  await addReply(user, connection, "Neji", "Happy journey")
  console.log("All the student introductions:")
  console.log(await getIntros(connection))
  const [lee_pda] = await web3.PublicKey.findProgramAddress([user.publicKey.toBuffer(), Buffer.from("Lee")], programId)
  console.log("All replies received by Brock:")
  console.log(await getReplies(connection, lee_pda))
}

main()
  .then(() => {
    console.log("Finished successfully")
    process.exit(0)
  })
  .catch((error) => {
    console.log(error)
    process.exit(1)
  })
