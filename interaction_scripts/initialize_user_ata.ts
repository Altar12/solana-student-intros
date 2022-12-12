/*
Each student receives 10 tokens for posting an intro
Each student receives 5 tokens to reply to a posted intro
This script is for initializing the token mint account i.e. only one time use script
*/
import * as web3 from "@solana/web3.js"
import * as token from "@solana/spl-token"
import { initializeKeypair } from "./initializeKeypair"
const programId = new web3.PublicKey("9nKhQhLdUq5z37SjiqmsNGGEXDUmLsoryek1gvWxUKsg")



async function main() {
    const connection = new web3.Connection("http://127.0.0.1:8899")
    const user = await initializeKeypair(connection)
    console.log("PublicKey:", user.publicKey.toBase58())
    let [mint] = await web3.PublicKey.findProgramAddress([Buffer.from("token_mint")], programId)
    let tokenAccount = await token.getAssociatedTokenAddress(mint, user.publicKey)
    let ix0 = token.createAssociatedTokenAccountInstruction(user.publicKey, tokenAccount, user.publicKey, mint)
    let txn = new web3.Transaction().add(ix0)
    await web3.sendAndConfirmTransaction(connection, txn, [user])
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